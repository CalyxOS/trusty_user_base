/*
 * This file is derived from src/sync/once.rs in the Rust standard library, used
 * under the Apache License, Version 2.0. Multi-thread support has been removed
 * until Trusty supports threading. The following is the original copyright
 * information from the Rust project:
 *
 * Copyrights in the Rust project are retained by their contributors. No
 * copyright assignment is required to contribute to the Rust project.
 *
 * Some files include explicit copyright notices and/or license notices.
 * For full authorship information, see the version control history or
 * https://thanks.rust-lang.org
 *
 * Except as otherwise noted (below and/or in individual files), Rust is
 * licensed under the Apache License, Version 2.0 <LICENSE-APACHE> or
 * <http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
 * <LICENSE-MIT> or <http://opensource.org/licenses/MIT>, at your option.
 *
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *      http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 */

//! A "once initialization" primitive
//!
//! This primitive is meant to be used to run one-time initialization. An
//! example use case would be for initializing an FFI library.
//!
//! This primitive is *not* thread-safe in the Trusty implementation, as
//! userspace threads are not yet supported.

// A "once" is a relatively simple primitive, and it's also typically provided
// by the OS as well (see `pthread_once` or `InitOnceExecuteOnce`). The OS
// primitives, however, tend to have surprising restrictions, such as the Unix
// one doesn't allow an argument to be passed to the function.
//
// As a result, we end up implementing it ourselves in the standard library.
// This also gives us the opportunity to optimize the implementation a bit which
// should help the fast path on call sites. Consequently, let's explain how this
// primitive works now!
//
// So to recap, the guarantees of a Once are that it will call the
// initialization closure at most once, and it will never return until the one
// that's running has finished running. This means that we need some form of
// blocking here while the custom callback is running at the very least.
// Additionally, we add on the restriction of **poisoning**. Whenever an
// initialization closure panics, the Once enters a "poisoned" state which means
// that all future calls will immediately panic as well.
//
// So to implement this, one might first reach for a `Mutex`, but those cannot
// be put into a `static`. It also gets a lot harder with poisoning to figure
// out when the mutex needs to be deallocated because it's not after the closure
// finishes, but after the first successful closure finishes.
//
// All in all, this is instead implemented with atomics and lock-free
// operations! Whee! Each `Once` has one word of atomic state, and this state is
// CAS'd on to determine what to do. There are four possible state of a `Once`:
//
// * Incomplete - no initialization has run yet, and no thread is currently
//                using the Once.
// * Poisoned - some thread has previously attempted to initialize the Once, but
//              it panicked, so the Once is now poisoned. There are no other
//              threads currently accessing this Once.
// * Running - some thread is currently attempting to run initialization. It may
//             succeed, so all future threads need to wait for it to finish.
//             Note that this state is accompanied with a payload, described
//             below.
// * Complete - initialization has completed and all future calls should finish
//              immediately.
//
// With 4 states we need 2 bits to encode this, and we use the remaining bits
// in the word we have allocated as a queue of threads waiting for the thread
// responsible for entering the RUNNING state. This queue is just a linked list
// of Waiter nodes which is monotonically increasing in size. Each node is
// allocated on the stack, and whenever the running closure finishes it will
// consume the entire queue and notify all waiters they should try again.
//
// You'll find a few more details in the implementation, but that's the gist of
// it!
//
// Atomic orderings:
// When running `Once` we deal with multiple atomics:
// `Once.state_and_queue` and an unknown number of `Waiter.signaled`.
// * `state_and_queue` is used (1) as a state flag, (2) for synchronizing the
//   result of the `Once`, and (3) for synchronizing `Waiter` nodes.
//     - At the end of the `call_inner` function we have to make sure the result
//       of the `Once` is acquired. So every load which can be the only one to
//       load COMPLETED must have at least Acquire ordering, which means all
//       three of them.
//     - `WaiterQueue::Drop` is the only place that may store COMPLETED, and
//       must do so with Release ordering to make the result available.
//     - `wait` inserts `Waiter` nodes as a pointer in `state_and_queue`, and
//       needs to make the nodes available with Release ordering. The load in
//       its `compare_exchange` can be Relaxed because it only has to compare
//       the atomic, not to read other data.
//     - `WaiterQueue::Drop` must see the `Waiter` nodes, so it must load
//       `state_and_queue` with Acquire ordering.
//     - There is just one store where `state_and_queue` is used only as a
//       state flag, without having to synchronize data: switching the state
//       from INCOMPLETE to RUNNING in `call_inner`. This store can be Relaxed,
//       but the read has to be Acquire because of the requirements mentioned
//       above.
// * `Waiter.signaled` is both used as a flag, and to protect a field with
//   interior mutability in `Waiter`. `Waiter.thread` is changed in
//   `WaiterQueue::Drop` which then sets `signaled` with Release ordering.
//   After `wait` loads `signaled` with Acquire and sees it is true, it needs to
//   see the changes to drop the `Waiter` struct correctly.
// * There is one place where the two atomics `Once.state_and_queue` and
//   `Waiter.signaled` come together, and might be reordered by the compiler or
//   processor. Because both use Acquire ordering such a reordering is not
//   allowed, so no need for SeqCst.

#[cfg(all(test, not(target_os = "emscripten")))]
mod tests;

use core::cell::Cell;
use core::fmt;
use core::sync::atomic::{AtomicUsize, Ordering};

/// A synchronization primitive which can be used to run a one-time global
/// initialization. Useful for one-time initialization for FFI or related
/// functionality. This type can only be constructed with [`Once::new()`].
///
/// This primitive is *not* thread-safe, as userpace threads are not yet
/// supported in Trusty.
///
/// # Examples
///
/// ```
/// use std::sync::Once;
///
/// static START: Once = Once::new();
///
/// START.call_once(|| {
///     // run initialization here
/// });
/// ```
pub struct Once {
    state: AtomicUsize,
}

/// State yielded to [`Once::call_once_force()`]’s closure parameter. The state
/// can be used to query the poison status of the [`Once`].
#[derive(Debug)]
pub struct OnceState {
    poisoned: bool,
    set_state_on_drop_to: Cell<usize>,
}

// Four states that a Once can be in, encoded into the lower bits of
// `state_and_queue` in the Once structure.
const INCOMPLETE: usize = 0x0;
const POISONED: usize = 0x1;
const RUNNING: usize = 0x2;
const COMPLETE: usize = 0x3;

// Mask to learn about the state. All other bits are the queue of waiters if
// this is in the RUNNING state.
impl Once {
    /// Creates a new `Once` value.
    #[inline]
    #[must_use]
    pub const fn new() -> Once {
        Once { state: AtomicUsize::new(INCOMPLETE) }
    }

    /// Performs an initialization routine once and only once. The given closure
    /// will be executed if this is the first time `call_once` has been called,
    /// and otherwise the routine will *not* be invoked.
    ///
    /// This method will block the calling thread if another initialization
    /// routine is currently running.
    ///
    /// When this function returns, it is guaranteed that some initialization
    /// has run and completed (it might not be the closure specified). It is also
    /// guaranteed that any memory writes performed by the executed closure can
    /// be reliably observed by other threads at this point (there is a
    /// happens-before relation between the closure and code executing after the
    /// return).
    ///
    /// If the given closure recursively invokes `call_once` on the same [`Once`]
    /// instance the exact behavior is not specified, allowed outcomes are
    /// a panic or a deadlock.
    ///
    /// # Examples
    ///
    /// ```
    /// use trusty_std::sync::Once;
    ///
    /// static mut VAL: usize = 0;
    /// static INIT: Once = Once::new();
    ///
    /// // Accessing a `static mut` is unsafe much of the time, but if we do so
    /// // in a synchronized fashion (e.g., write once or read all) then we're
    /// // good to go!
    /// //
    /// // This function will only call `expensive_computation` once, and will
    /// // otherwise always return the value returned from the first invocation.
    /// fn get_cached_val() -> usize {
    ///     unsafe {
    ///         INIT.call_once(|| {
    ///             VAL = expensive_computation();
    ///         });
    ///         VAL
    ///     }
    /// }
    ///
    /// fn expensive_computation() -> usize {
    ///     // ...
    /// # 2
    /// }
    /// ```
    ///
    /// # Panics
    ///
    /// The closure `f` will only be executed once if this is called
    /// concurrently amongst many threads. If that closure panics, however, then
    /// it will *poison* this [`Once`] instance, causing all future invocations of
    /// `call_once` to also panic.
    ///
    /// This is similar to [poisoning with mutexes][poison].
    ///
    /// [poison]: struct.Mutex.html#poisoning
    pub fn call_once<F>(&self, f: F)
    where
        F: FnOnce(),
    {
        // Fast path check
        if self.is_completed() {
            return;
        }

        let mut f = Some(f);
        self.call_inner(false, &mut |_| f.take().unwrap()());
    }

    /// Performs the same function as [`call_once()`] except ignores poisoning.
    ///
    /// Unlike [`call_once()`], if this [`Once`] has been poisoned (i.e., a previous
    /// call to [`call_once()`] or [`call_once_force()`] caused a panic), calling
    /// [`call_once_force()`] will still invoke the closure `f` and will _not_
    /// result in an immediate panic. If `f` panics, the [`Once`] will remain
    /// in a poison state. If `f` does _not_ panic, the [`Once`] will no
    /// longer be in a poison state and all future calls to [`call_once()`] or
    /// [`call_once_force()`] will be no-ops.
    ///
    /// The closure `f` is yielded a [`OnceState`] structure which can be used
    /// to query the poison status of the [`Once`].
    ///
    /// [`call_once()`]: Once::call_once
    /// [`call_once_force()`]: Once::call_once_force
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Once;
    /// use std::thread;
    ///
    /// static INIT: Once = Once::new();
    ///
    /// // poison the once
    /// let handle = thread::spawn(|| {
    ///     INIT.call_once(|| panic!());
    /// });
    /// assert!(handle.join().is_err());
    ///
    /// // poisoning propagates
    /// let handle = thread::spawn(|| {
    ///     INIT.call_once(|| {});
    /// });
    /// assert!(handle.join().is_err());
    ///
    /// // call_once_force will still run and reset the poisoned state
    /// INIT.call_once_force(|state| {
    ///     assert!(state.is_poisoned());
    /// });
    ///
    /// // once any success happens, we stop propagating the poison
    /// INIT.call_once(|| {});
    /// ```
    pub fn call_once_force<F>(&self, f: F)
    where
        F: FnOnce(&OnceState),
    {
        // Fast path check
        if self.is_completed() {
            return;
        }

        let mut f = Some(f);
        self.call_inner(true, &mut |p| f.take().unwrap()(p));
    }

    /// Returns `true` if some [`call_once()`] call has completed
    /// successfully. Specifically, `is_completed` will return false in
    /// the following situations:
    ///   * [`call_once()`] was not called at all,
    ///   * [`call_once()`] was called, but has not yet completed,
    ///   * the [`Once`] instance is poisoned
    ///
    /// This function returning `false` does not mean that [`Once`] has not been
    /// executed. For example, it may have been executed in the time between
    /// when `is_completed` starts executing and when it returns, in which case
    /// the `false` return value would be stale (but still permissible).
    ///
    /// [`call_once()`]: Once::call_once
    ///
    /// # Examples
    ///
    /// ```
    /// use std::sync::Once;
    ///
    /// static INIT: Once = Once::new();
    ///
    /// assert_eq!(INIT.is_completed(), false);
    /// INIT.call_once(|| {
    ///     assert_eq!(INIT.is_completed(), false);
    /// });
    /// assert_eq!(INIT.is_completed(), true);
    /// ```
    ///
    /// ```
    /// use std::sync::Once;
    /// use std::thread;
    ///
    /// static INIT: Once = Once::new();
    ///
    /// assert_eq!(INIT.is_completed(), false);
    /// let handle = thread::spawn(|| {
    ///     INIT.call_once(|| panic!());
    /// });
    /// assert!(handle.join().is_err());
    /// assert_eq!(INIT.is_completed(), false);
    /// ```
    #[inline]
    pub fn is_completed(&self) -> bool {
        // An `Acquire` load is enough because that makes all the initialization
        // operations visible to us, and, this being a fast path, weaker
        // ordering helps with performance. This `Acquire` synchronizes with
        // `Release` operations on the slow path.
        self.state.load(Ordering::Acquire) == COMPLETE
    }

    // This is a non-generic function to reduce the monomorphization cost of
    // using `call_once` (this isn't exactly a trivial or small implementation).
    //
    // Additionally, this is tagged with `#[cold]` as it should indeed be cold
    // and it helps let LLVM know that calls to this function should be off the
    // fast path. Essentially, this should help generate more straight line code
    // in LLVM.
    //
    // Finally, this takes an `FnMut` instead of a `FnOnce` because there's
    // currently no way to take an `FnOnce` and call it via virtual dispatch
    // without some allocation overhead.
    #[cold]
    fn call_inner(&self, ignore_poisoning: bool, init: &mut dyn FnMut(&OnceState)) {
        let mut state = self.state.load(Ordering::Acquire);
        loop {
            match state {
                COMPLETE => break,
                POISONED if !ignore_poisoning => {
                    // Panic to propagate the poison.
                    panic!("Once instance has previously been poisoned");
                }
                POISONED | INCOMPLETE => {
                    // Try to register this thread as the one RUNNING.
                    let exchange_result = self.state.compare_exchange(
                        state,
                        RUNNING,
                        Ordering::Acquire,
                        Ordering::Acquire,
                    );
                    if let Err(old) = exchange_result {
                        state = old;
                        continue;
                    }
                    // Run the initialization function, letting it know if we're
                    // poisoned or not.
                    let init_state = OnceState {
                        poisoned: state == POISONED,
                        set_state_on_drop_to: Cell::new(COMPLETE),
                    };
                    init(&init_state);
                    self.state.store(COMPLETE, Ordering::Release);
                    break;
                }
                _ => {
                    // All other values must be RUNNING with possibly a
                    // pointer to the waiter queue in the more significant bits.
                    unimplemented!("Trusty does not yet support threads.");
                }
            }
        }
    }
}

impl fmt::Debug for Once {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Once").finish_non_exhaustive()
    }
}

impl OnceState {
    /// Returns `true` if the associated [`Once`] was poisoned prior to the
    /// invocation of the closure passed to [`Once::call_once_force()`].
    ///
    /// # Examples
    ///
    /// A poisoned [`Once`]:
    ///
    /// ```
    /// use std::sync::Once;
    /// use std::thread;
    ///
    /// static INIT: Once = Once::new();
    ///
    /// // poison the once
    /// let handle = thread::spawn(|| {
    ///     INIT.call_once(|| panic!());
    /// });
    /// assert!(handle.join().is_err());
    ///
    /// INIT.call_once_force(|state| {
    ///     assert!(state.is_poisoned());
    /// });
    /// ```
    ///
    /// An unpoisoned [`Once`]:
    ///
    /// ```
    /// use std::sync::Once;
    ///
    /// static INIT: Once = Once::new();
    ///
    /// INIT.call_once_force(|state| {
    ///     assert!(!state.is_poisoned());
    /// });
    #[allow(unused)]
    pub fn is_poisoned(&self) -> bool {
        self.poisoned
    }

    /// Poison the associated [`Once`] without explicitly panicking.
    // NOTE: This is currently only exposed for the `lazy` module
    #[allow(unused)]
    pub(crate) fn poison(&self) {
        self.set_state_on_drop_to.set(POISONED);
    }
}

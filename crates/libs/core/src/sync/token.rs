// ------------------------------------------------------------
// Copyright (c) Microsoft Corporation.  All rights reserved.
// Licensed under the MIT License (MIT). See License.txt in the repo root for license information.
// ------------------------------------------------------------

use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};

#[derive(Clone)]
pub struct CancellationToken {
    inner: Arc<TokenInner>,
}

struct TokenInner {
    cancelled: AtomicBool,
    wakers: Mutex<Vec<Waker>>,
}

impl CancellationToken {
    pub fn new() -> Self {
        CancellationToken {
            inner: Arc::new(TokenInner {
                cancelled: AtomicBool::new(false),
                wakers: Mutex::new(Vec::new()),
            }),
        }
    }

    pub fn cancel(&self) {
        // Set the cancelled flag
        self.inner.cancelled.store(true, Ordering::Release);
        
        // Wake all waiting tasks
        let mut wakers = self.inner.wakers.lock().unwrap();
        for waker in wakers.drain(..) {
            waker.wake();
        }
    }

    pub fn is_cancelled(&self) -> bool {
        self.inner.cancelled.load(Ordering::Acquire)
    }

    /// Returns a future that completes when cancellation is triggered
    pub fn cancelled(&self) -> CancelledFuture {
        CancelledFuture {
            token: self.clone(),
        }
    }
}

pub struct CancelledFuture {
    token: CancellationToken,
}

impl Future for CancelledFuture {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.token.is_cancelled() {
            return Poll::Ready(());
        }

        // Register this task's waker to be notified when cancelled
        let mut wakers = self.token.inner.wakers.lock().unwrap();
        
        // Double-check after acquiring the lock
        if self.token.is_cancelled() {
            return Poll::Ready(());
        }
        
        // Store the waker to be called when cancel() is invoked
        wakers.push(cx.waker().clone());
        Poll::Pending
    }
}

impl Default for CancellationToken {
    fn default() -> Self {
        Self::new()
    }
}
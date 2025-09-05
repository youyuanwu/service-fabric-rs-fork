// ------------------------------------------------------------
// Copyright (c) Microsoft Corporation.  All rights reserved.
// Licensed under the MIT License (MIT). See License.txt in the repo root for license information.
// ------------------------------------------------------------

use std::{
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};

use crate::sync::FabricReceiver;

pub trait FabricFuture: Future {
    // TODO: inner reveiver blocking wait is not supported.
    //  fn blocking_wait(self) -> Self::Output;
}

impl<T, R, F> FabricFuture for FabricFutureImpl<T, R, F>
where
    F: FnOnce(T) -> R + Unpin,
{
    //     fn blocking_wait(self) -> Self::Output {
    //         // Implement blocking wait logic here
    //         self.inner.
    //     }
}

pub(crate) struct FabricFutureImpl<T, R, F>
where
    F: FnOnce(T) -> R,
{
    inner: FabricReceiver<crate::WinResult<T>>,
    mapper: Option<F>,
}

impl<T, R, F> FabricFutureImpl<T, R, F>
where
    F: FnOnce(T) -> R,
{
    pub fn new(inner: FabricReceiver<crate::WinResult<T>>, mapper: F) -> Self {
        Self {
            inner,
            mapper: Some(mapper),
        }
    }
}

impl<T, R, F> Future for FabricFutureImpl<T, R, F>
where
    F: FnOnce(T) -> R + Unpin,
{
    type Output = crate::Result<R>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // Use get_mut to access the inner field mutably without moving
        let this = self.get_mut();
        // Pin the inner FabricReceiver and poll it
        let pinned = Pin::new(&mut this.inner);
        pinned
            .poll(cx)
            .map(|res| {
                match res {
                    Ok(val) => {
                        // Use take to move the mapper out, leaving None in its place
                        let mapper = this.mapper.take().expect("Mapper already taken");
                        match val {
                            Ok(v) => Ok(mapper(v)),
                            Err(e) => Err(crate::Error::from(e)),
                        }
                    }
                    Err(e) => Err(crate::Error::from(e)),
                }
            })
            .map_err(Into::into)
    }
}

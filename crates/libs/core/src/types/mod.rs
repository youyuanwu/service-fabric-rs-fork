// ------------------------------------------------------------
// Copyright (c) Microsoft Corporation.  All rights reserved.
// Licensed under the MIT License (MIT). See License.txt in the repo root for license information.
// ------------------------------------------------------------

// This mod consists of type conversion of com api with safe rust wrappers.
mod common;
pub use common::*;
mod client;
pub use client::*;
mod runtime;
pub use runtime::{EndpointResourceDescription, health::*, stateful::*, store::*};

#[cfg(test)]
mod mockifabricclientsettings;

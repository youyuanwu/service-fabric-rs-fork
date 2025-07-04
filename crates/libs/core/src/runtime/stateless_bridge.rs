// ------------------------------------------------------------
// Copyright (c) Microsoft Corporation.  All rights reserved.
// Licensed under the MIT License (MIT). See License.txt in the repo root for license information.
// ------------------------------------------------------------

// windows::core::implement macro generates snake case types.
#![allow(non_camel_case_types)]

use std::sync::Arc;

use crate::{runtime::StatelessServicePartition, strings::WStringWrap, sync::BridgeContext};
use mssf_com::{
    FabricCommon::IFabricStringResult,
    FabricRuntime::{
        IFabricStatelessServiceFactory, IFabricStatelessServiceFactory_Impl,
        IFabricStatelessServiceInstance, IFabricStatelessServiceInstance_Impl,
        IFabricStatelessServicePartition,
    },
    FabricTypes::FABRIC_URI,
};
use windows_core::implement;

use super::{
    executor::Executor,
    stateless::{StatelessServiceFactory, StatelessServiceInstance},
};

#[implement(IFabricStatelessServiceFactory)]
pub struct StatelessServiceFactoryBridge<E, F>
where
    E: Executor + 'static,
    F: StatelessServiceFactory + 'static,
{
    inner: F,
    rt: E,
}

impl<E, F> StatelessServiceFactoryBridge<E, F>
where
    E: Executor,
    F: StatelessServiceFactory,
{
    pub fn create(factory: F, rt: E) -> StatelessServiceFactoryBridge<E, F> {
        StatelessServiceFactoryBridge::<E, F> { inner: factory, rt }
    }
}

impl<E, F> IFabricStatelessServiceFactory_Impl for StatelessServiceFactoryBridge_Impl<E, F>
where
    E: Executor,
    F: StatelessServiceFactory,
{
    #[allow(clippy::not_unsafe_ptr_arg_deref)]
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip_all, ret(level = "debug"), err)
    )]
    fn CreateInstance(
        &self,
        servicetypename: &crate::PCWSTR,
        servicename: FABRIC_URI,
        initializationdatalength: u32,
        initializationdata: *const u8,
        partitionid: &crate::GUID,
        instanceid: i64,
    ) -> crate::WinResult<IFabricStatelessServiceInstance> {
        let h_servicename = WStringWrap::from(crate::PCWSTR(servicename.0)).into();
        let h_servicetypename = WStringWrap::from(*servicetypename).into();
        let data = unsafe {
            if !initializationdata.is_null() {
                std::slice::from_raw_parts(initializationdata, initializationdatalength as usize)
            } else {
                &[]
            }
        };

        let instance = self.inner.create_instance(
            &h_servicetypename,
            &h_servicename,
            data,
            partitionid,
            instanceid,
        )?;
        let rt = self.rt.clone();
        let instance_bridge = IFabricStatelessServiceInstanceBridge::create(instance, rt);

        Ok(instance_bridge.into())
    }
}

// bridge from safe service instance to com
#[implement(IFabricStatelessServiceInstance)]

struct IFabricStatelessServiceInstanceBridge<E, S>
where
    E: Executor,
    S: StatelessServiceInstance + 'static,
{
    inner: Arc<S>,
    rt: E,
}

impl<E, S> IFabricStatelessServiceInstanceBridge<E, S>
where
    E: Executor,
    S: StatelessServiceInstance,
{
    pub fn create(instance: S, rt: E) -> IFabricStatelessServiceInstanceBridge<E, S>
    where
        S: StatelessServiceInstance,
    {
        IFabricStatelessServiceInstanceBridge {
            inner: Arc::new(instance),
            rt,
        }
    }
}

impl<E, S> IFabricStatelessServiceInstance_Impl for IFabricStatelessServiceInstanceBridge_Impl<E, S>
where
    E: Executor,
    S: StatelessServiceInstance + 'static,
{
    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip_all, ret(level = "debug"), err)
    )]
    fn BeginOpen(
        &self,
        partition: windows_core::Ref<IFabricStatelessServicePartition>,
        callback: windows_core::Ref<super::IFabricAsyncOperationCallback>,
    ) -> crate::WinResult<super::IFabricAsyncOperationContext> {
        let partition_cp = partition.unwrap().clone();
        let partition_bridge = StatelessServicePartition::new(partition_cp);
        let inner = self.inner.clone();
        let (ctx, token) = BridgeContext::make(callback);
        ctx.spawn(&self.rt, async move {
            inner
                .open(&partition_bridge, token)
                .await
                .map(|s| IFabricStringResult::from(WStringWrap::from(s)))
                .map_err(crate::WinError::from)
        })
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip_all, ret(level = "debug"), err)
    )]
    fn EndOpen(
        &self,
        context: windows_core::Ref<super::IFabricAsyncOperationContext>,
    ) -> crate::WinResult<IFabricStringResult> {
        BridgeContext::result(context)?
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip_all, ret(level = "debug"), err)
    )]
    fn BeginClose(
        &self,
        callback: windows_core::Ref<super::IFabricAsyncOperationCallback>,
    ) -> crate::WinResult<super::IFabricAsyncOperationContext> {
        let inner = self.inner.clone();
        let (ctx, token) = BridgeContext::make(callback);
        ctx.spawn(&self.rt, async move {
            inner.close(token).await.map_err(crate::WinError::from)
        })
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip_all, ret(level = "debug"), err)
    )]
    fn EndClose(
        &self,
        context: windows_core::Ref<super::IFabricAsyncOperationContext>,
    ) -> crate::WinResult<()> {
        BridgeContext::result(context)?
    }

    #[cfg_attr(
        feature = "tracing",
        tracing::instrument(skip_all, ret(level = "debug"))
    )]
    fn Abort(&self) {
        self.inner.abort()
    }
}

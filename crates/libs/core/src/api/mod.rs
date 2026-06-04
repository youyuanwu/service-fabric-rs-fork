//! Dynamically load SF libs and c functions.
//! SF shared lib provides these functions, and we dynamically load them here so that user of this crate
//! does not need to worry about installing SF lib and linking, which can be complex.
//!

use std::path::Path;

use mssf_com::{
    FabricClient::{IFabricClientConnectionEventHandler, IFabricServiceNotificationEventHandler},
    FabricCommon::{
        IFabricAsyncOperationCallback, IFabricAsyncOperationContext, IFabricStringResult,
    },
    FabricRuntime::IFabricStoreEventHandler,
    FabricTypes::{FABRIC_CLIENT_ROLE, FABRIC_LOCAL_STORE_KIND, FABRIC_REPLICATOR_SETTINGS},
};
use windows_core::{Interface, Param};

/// If user specifies this env variable, we will try to load SF libs from the path specified by this env variable.
const MSSF_SF_BIN_PATH_ENV_NAME: &str = "MSSF_SF_BIN_PATH";

lazy_static::lazy_static! {
    static ref LIB_TABLE: LibTable = LibTable::create();
    /// All SF APIs entrypoints needed for mssf.
    /// These APIs are lazy loaded at the first time use after app starts.
    pub static ref API_TABLE: ApiTable = ApiTable::create(&LIB_TABLE);
}

/// Contains all the SF shared libs needs to be loaded for mssf.
pub struct LibTable {
    fabric_runtime: libloading::Library,
    fabric_common: libloading::Library,
    fabric_client: libloading::Library,
}

impl LibTable {
    fn create() -> Self {
        // Read the env variable to get SF bin path, and try to load SF libs from that path.
        // If the env variable is not set or loading from that path fails, fallback to load SF libs from system path.
        let sf_bin_abs = std::env::var(MSSF_SF_BIN_PATH_ENV_NAME)
            .ok()
            .map(std::path::PathBuf::from);
        Self {
            fabric_runtime: load_lib("FabricRuntime", sf_bin_abs.as_deref()),
            fabric_common: load_lib("FabricCommon", sf_bin_abs.as_deref()),
            fabric_client: load_lib("FabricClient", sf_bin_abs.as_deref()),
        }
    }
}

fn load_lib(name: &str, sf_bin_abs: Option<&Path>) -> libloading::Library {
    let filename = match sf_bin_abs {
        Some(path) => path.join(libloading::library_filename(name)),
        None => libloading::library_filename(name).into(),
    };
    // On Windows, loading a DLL by absolute path does not add its directory to the search
    // path for its own transitive dependencies. Use LOAD_WITH_ALTERED_SEARCH_PATH (0x8) so
    // that sibling DLLs in the SF bin directory can be resolved.
    #[cfg(windows)]
    let lib = {
        use libloading::os::windows::{LOAD_WITH_ALTERED_SEARCH_PATH, Library};
        let flags = if sf_bin_abs.is_some() {
            LOAD_WITH_ALTERED_SEARCH_PATH
        } else {
            0
        };
        unsafe { Library::load_with_flags(&filename, flags) }.map(libloading::Library::from)
    };
    #[cfg(not(windows))]
    let lib = unsafe { libloading::Library::new(&filename) };
    lib.unwrap_or_else(|e| panic!("cannot load lib {filename:?} :{e}"))
}

fn load_fn<T>(lib: &'static libloading::Library, name: &str) -> libloading::Symbol<'static, T> {
    unsafe { lib.get(name.as_bytes()) }.unwrap_or_else(|e| panic!("cannot load fn {name} :{e}"))
}

/// Contains all SF APIs loaded from SF libs needed for mssf.
/// More APIs can be added here when mssf needs them.
#[allow(non_snake_case)]
pub struct ApiTable {
    fabric_get_last_error_message_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(message: *mut *mut core::ffi::c_void) -> crate::HRESULT,
    >,
    fabric_create_client3_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(
            connectionstringssize: u16,
            connectionstrings: *const windows_core::PCWSTR,
            __midl__fabricclientmodule0002: *mut core::ffi::c_void,
            __midl__fabricclientmodule0003: *mut core::ffi::c_void,
            iid: *const windows_core::GUID,
            fabricclient: *mut *mut core::ffi::c_void,
        ) -> crate::HRESULT,
    >,
    fabric_create_local_client3_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(
            __midl__fabricclientmodule0004: *mut core::ffi::c_void,
            __midl__fabricclientmodule0005: *mut core::ffi::c_void,
            iid: *const windows_core::GUID,
            fabricclient: *mut *mut core::ffi::c_void,
        ) -> crate::HRESULT,
    >,

    fabric_create_local_client4_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(
            __midl__fabricclientmodule0006: *mut core::ffi::c_void,
            __midl__fabricclientmodule0007: *mut core::ffi::c_void,
            clientrole: FABRIC_CLIENT_ROLE,
            iid: *const windows_core::GUID,
            fabricclient: *mut *mut core::ffi::c_void,
        ) -> crate::HRESULT,
    >,

    fabric_create_runtime_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(
            riid: *const windows_core::GUID,
            fabricruntime: *mut *mut core::ffi::c_void,
        ) -> crate::HRESULT,
    >,

    fabric_get_activation_context_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(
            riid: *const windows_core::GUID,
            activationcontext: *mut *mut core::ffi::c_void,
        ) -> crate::HRESULT,
    >,

    fabric_begin_get_node_context_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(
            timeoutmilliseconds: u32,
            callback: *mut core::ffi::c_void,
            context: *mut *mut core::ffi::c_void,
        ) -> crate::HRESULT,
    >,

    fabric_end_get_node_context_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(
            context: *mut core::ffi::c_void,
            nodecontext: *mut *mut core::ffi::c_void,
        ) -> crate::HRESULT,
    >,
    fabric_get_node_context_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(nodecontext: *mut *mut core::ffi::c_void) -> crate::HRESULT,
    >,
    fabric_create_key_value_store_replica_fn: libloading::Symbol<
        'static,
        unsafe extern "system" fn(
            riid: *const windows_core::GUID,
            storename: windows_core::PCWSTR,
            partitionid: windows_core::GUID,
            replicaid: i64,
            replicatorsettings: *const FABRIC_REPLICATOR_SETTINGS,
            localstorekind: FABRIC_LOCAL_STORE_KIND,
            localstoresettings: *const core::ffi::c_void,
            storeeventhandler: *mut core::ffi::c_void,
            keyvaluestore: *mut *mut core::ffi::c_void,
        ) -> crate::HRESULT,
    >,
}

impl ApiTable {
    fn create(lib_table: &'static LibTable) -> Self {
        Self {
            fabric_get_last_error_message_fn: load_fn(
                &lib_table.fabric_common,
                "FabricGetLastErrorMessage",
            ),
            fabric_create_client3_fn: load_fn(&lib_table.fabric_client, "FabricCreateClient3"),
            fabric_create_local_client3_fn: load_fn(
                &lib_table.fabric_client,
                "FabricCreateLocalClient3",
            ),
            fabric_create_local_client4_fn: load_fn(
                &lib_table.fabric_client,
                "FabricCreateLocalClient4",
            ),
            fabric_create_runtime_fn: load_fn(&lib_table.fabric_runtime, "FabricCreateRuntime"),
            fabric_get_activation_context_fn: load_fn(
                &lib_table.fabric_runtime,
                "FabricGetActivationContext",
            ),
            fabric_begin_get_node_context_fn: load_fn(
                &lib_table.fabric_runtime,
                "FabricBeginGetNodeContext",
            ),
            fabric_end_get_node_context_fn: load_fn(
                &lib_table.fabric_runtime,
                "FabricEndGetNodeContext",
            ),
            fabric_get_node_context_fn: load_fn(&lib_table.fabric_runtime, "FabricGetNodeContext"),
            fabric_create_key_value_store_replica_fn: load_fn(
                &lib_table.fabric_runtime,
                "FabricCreateKeyValueStoreReplica",
            ),
        }
    }

    pub fn fabric_get_last_error_message(&self) -> crate::WinResult<IFabricStringResult> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe { (self.fabric_get_last_error_message_fn)(std::ptr::addr_of_mut!(result)) }.ok()?;
        assert!(!result.is_null());
        Ok(unsafe { IFabricStringResult::from_raw(result) })
    }

    pub fn fabric_create_client3<T: Interface>(
        &self,
        connectionstrings: &[windows_core::PCWSTR],
        service_notification_handler: Option<&IFabricServiceNotificationEventHandler>,
        client_connection_handler: Option<&IFabricClientConnectionEventHandler>,
    ) -> crate::WinResult<T> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe {
            (self.fabric_create_client3_fn)(
                connectionstrings.len().try_into().unwrap(),
                connectionstrings.as_ptr(),
                service_notification_handler.param().abi(),
                client_connection_handler.param().abi(),
                &T::IID,
                std::ptr::addr_of_mut!(result),
            )
        }
        .ok()?;
        Ok(unsafe { T::from_raw(result) })
    }

    pub fn fabric_create_local_client3<T: Interface>(
        &self,
        service_notification_handler: Option<&IFabricServiceNotificationEventHandler>,
        client_connection_handler: Option<&IFabricClientConnectionEventHandler>,
    ) -> crate::WinResult<T> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe {
            (self.fabric_create_local_client3_fn)(
                service_notification_handler.param().abi(),
                client_connection_handler.param().abi(),
                &T::IID,
                std::ptr::addr_of_mut!(result),
            )
        }
        .ok()?;
        Ok(unsafe { T::from_raw(result) })
    }

    pub fn fabric_create_local_client4<T: Interface>(
        &self,
        service_notification_handler: Option<&IFabricServiceNotificationEventHandler>,
        client_connection_handler: Option<&IFabricClientConnectionEventHandler>,
        clientrole: FABRIC_CLIENT_ROLE,
    ) -> crate::WinResult<T> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe {
            (self.fabric_create_local_client4_fn)(
                service_notification_handler.param().abi(),
                client_connection_handler.param().abi(),
                clientrole,
                &T::IID,
                std::ptr::addr_of_mut!(result),
            )
        }
        .ok()?;
        Ok(unsafe { T::from_raw(result) })
    }

    pub fn fabric_create_runtime<T: Interface>(&self) -> crate::WinResult<T> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe { (self.fabric_create_runtime_fn)(&T::IID, std::ptr::addr_of_mut!(result)) }.ok()?;
        Ok(unsafe { T::from_raw(result) })
    }

    pub fn fabric_get_activation_context<T: Interface>(&self) -> crate::WinResult<T> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe { (self.fabric_get_activation_context_fn)(&T::IID, std::ptr::addr_of_mut!(result)) }
            .ok()?;
        Ok(unsafe { T::from_raw(result) })
    }

    pub fn fabric_begin_get_node_context(
        &self,
        timeoutmilliseconds: u32,
        callback: Option<&IFabricAsyncOperationCallback>,
    ) -> crate::WinResult<IFabricAsyncOperationContext> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe {
            (self.fabric_begin_get_node_context_fn)(
                timeoutmilliseconds,
                callback.param().abi(),
                std::ptr::addr_of_mut!(result),
            )
        }
        .ok()?;
        Ok(unsafe { IFabricAsyncOperationContext::from_raw(result) })
    }

    pub fn fabric_end_get_node_context<T: Interface>(
        &self,
        context: Option<&IFabricAsyncOperationContext>,
    ) -> crate::WinResult<T> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe {
            (self.fabric_end_get_node_context_fn)(
                context.param().abi(),
                std::ptr::addr_of_mut!(result),
            )
        }
        .ok()?;
        Ok(unsafe { T::from_raw(result) })
    }

    pub fn fabric_get_node_context<T: Interface>(&self) -> crate::WinResult<T> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe { (self.fabric_get_node_context_fn)(std::ptr::addr_of_mut!(result)) }.ok()?;
        Ok(unsafe { T::from_raw(result) })
    }

    #[allow(clippy::too_many_arguments)]
    pub fn fabric_create_key_value_store_replica<T: Interface>(
        &self,
        storename: windows_core::PCWSTR,
        partitionid: windows_core::GUID,
        replicaid: i64,
        replicatorsettings: *const FABRIC_REPLICATOR_SETTINGS,
        localstorekind: FABRIC_LOCAL_STORE_KIND,
        localstoresettings: *const core::ffi::c_void,
        storeeventhandler: Option<&IFabricStoreEventHandler>,
    ) -> crate::WinResult<T> {
        let mut result = std::ptr::null_mut::<core::ffi::c_void>();
        unsafe {
            (self.fabric_create_key_value_store_replica_fn)(
                &T::IID,
                storename,
                partitionid,
                replicaid,
                replicatorsettings,
                localstorekind,
                localstoresettings,
                storeeventhandler.param().abi(),
                std::ptr::addr_of_mut!(result),
            )
        }
        .ok()?;
        Ok(unsafe { T::from_raw(result) })
    }
}

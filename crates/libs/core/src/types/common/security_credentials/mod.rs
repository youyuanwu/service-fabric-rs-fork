// ------------------------------------------------------------
// Copyright (c) Microsoft Corporation.  All rights reserved.
// Licensed under the MIT License (MIT). See License.txt in the repo root for license information.
// ------------------------------------------------------------
use mssf_com::FabricClient::IFabricClientSettings2;

mod claims_credentials;
pub use claims_credentials::*;
mod fabric_protection_level;
pub use fabric_protection_level::*;
mod windows_credentials;
pub use windows_credentials::*;
mod x509_credentials;
pub use x509_credentials::*;

/// Idiomatic FABRIC_SECURITY_CREDENTIALS wrapper
/// Currently, just a placeholder
#[non_exhaustive]
pub enum FabricSecurityCredentials {
    // TODO: consider None (to clear previously set settings), X509Credentials2?
    Windows(FabricWindowsCredentials),
    X509(FabricX509Credentials),
    //FabricX509Credentials2(FabricX509Credentials2),
    Claims(FabricClaimsCredentials),
}

trait FabricSecurityCredentialKind {
    fn apply_inner(&self, settings_interface: IFabricClientSettings2) -> crate::Result<()>;
}

impl FabricSecurityCredentials {
    pub fn apply(&self, settings_interface: IFabricClientSettings2) -> crate::Result<()> {
        match &self {
            FabricSecurityCredentials::X509(v) => v as &dyn FabricSecurityCredentialKind,
            FabricSecurityCredentials::Claims(v) => v as &dyn FabricSecurityCredentialKind,
            FabricSecurityCredentials::Windows(v) => v as &dyn FabricSecurityCredentialKind,
        }
        .apply_inner(settings_interface)
    }
}

#[cfg(test)]
pub(crate) mod test {}

// ------------------------------------------------------------
// Copyright (c) Microsoft Corporation.  All rights reserved.
// Licensed under the MIT License (MIT). See License.txt in the repo root for license information.
// ------------------------------------------------------------

use mssf_com::FabricCommon::{IFabricStringResult, IFabricStringResult_Impl};
use windows_core::{implement, HSTRING, PCWSTR};

// Basic implementation of fabric string result
// usually used as string return value to fabric runtime.
#[derive(Debug)]
#[implement(IFabricStringResult)]
pub struct StringResult {
    data: HSTRING,
}

// Recommend to use HSTRINGWrap to construct this and convert to
// IFabricStringResult.
impl StringResult {
    pub fn new(data: HSTRING) -> StringResult {
        StringResult { data }
    }
}

impl IFabricStringResult_Impl for StringResult {
    fn get_String(&self) -> windows::core::PCWSTR {
        // This is some hack to get the raw pointer out.
        windows::core::PCWSTR::from_raw(self.data.as_ptr())
    }
}

// If nullptr returns empty string.
// requires the PCWSTR points to a valid buffer with null terminatior
fn safe_pwstr_to_hstring(raw: PCWSTR) -> HSTRING {
    if raw.is_null() {
        return HSTRING::new();
    }
    HSTRING::from_wide(unsafe { raw.as_wide() }).unwrap()
}

// Convert helper for HSTRING and PCWSTR and IFabricStringResult
pub struct HSTRINGWrap {
    h: HSTRING,
}

impl From<HSTRING> for HSTRINGWrap {
    fn from(value: HSTRING) -> Self {
        Self { h: value }
    }
}

impl From<PCWSTR> for HSTRINGWrap {
    fn from(value: PCWSTR) -> Self {
        let h = safe_pwstr_to_hstring(value);
        Self { h }
    }
}

impl From<HSTRINGWrap> for HSTRING {
    fn from(val: HSTRINGWrap) -> Self {
        val.h
    }
}

impl From<&IFabricStringResult> for HSTRINGWrap {
    fn from(value: &IFabricStringResult) -> Self {
        let content = unsafe { value.get_String() };
        let h = safe_pwstr_to_hstring(content);
        Self { h }
    }
}

impl From<HSTRINGWrap> for IFabricStringResult {
    fn from(value: HSTRINGWrap) -> Self {
        StringResult::new(value.h).into()
    }
}

// note that hstring must be valid for pcwstr lifetime
pub fn get_pcwstr_from_opt(opt: &Option<HSTRING>) -> PCWSTR {
    match opt {
        Some(x) => PCWSTR(x.as_ptr()),
        None => PCWSTR::null(),
    }
}

#[cfg(test)]
mod test {
    use crate::strings::HSTRINGWrap;

    use super::StringResult;
    use mssf_com::FabricCommon::IFabricStringResult;
    use windows_core::HSTRING;

    #[test]
    fn test_str_addr() {
        // Test the addr returned to SF is right.
        let addr = "1.2.3.4:1234";

        // Check hstring len.
        let haddr = HSTRING::from(addr);
        let haddr_slice = haddr.as_wide();
        assert_eq!(haddr_slice.len(), 12);

        // check StringResult len.
        let com_addr: IFabricStringResult = StringResult::new(haddr.clone()).into();
        let raw = unsafe { com_addr.get_String() };
        let slice = unsafe { raw.as_wide() };
        assert_eq!(slice.len(), 12);

        // check StringResult conversion is right
        let haddr2: HSTRING = HSTRINGWrap::from(&com_addr).into();
        assert_eq!(haddr, haddr2);
    }
}

#[cfg(test)]
mod com_test {
    use std::{
        cell::Cell,
        sync::{Arc, Mutex},
        time::Duration,
    };

    use mssf_com::FabricCommon::{IFabricStringResult, IFabricStringResult_Impl};
    use windows_core::{implement, AsImpl, HSTRING, PCWSTR};

    #[implement(IFabricStringResult)]
    pub struct CountingString {
        data: HSTRING,
        pub counter: Counter,
    }

    impl CountingString {
        pub fn new(data: HSTRING, counter: Counter) -> Self {
            Self { data, counter }
        }

        pub fn my_action(self, h: tokio::runtime::Handle) -> IFabricStringResult {
            assert!(self.counter.get_count() >= 1);
            let com1: IFabricStringResult = self.into();
            let com2 = com1.clone();
            h.spawn(async move {
                tokio::time::sleep(Duration::from_millis(10)).await;
                let s: &CountingString = unsafe { com2.as_impl() };
                assert_eq!(s.counter.get_count(), 2);
            });
            com1
        }
    }

    impl IFabricStringResult_Impl for CountingString {
        fn get_String(&self) -> PCWSTR {
            PCWSTR(self.data.as_ptr())
        }
    }

    // The count value is the number of copies it currently has.
    pub struct Counter {
        count: Arc<Mutex<Cell<u32>>>,
    }

    impl Default for Counter {
        // New counter has value 1.
        fn default() -> Self {
            Self {
                count: Arc::new(Mutex::new(Cell::new(1))),
            }
        }
    }

    impl Counter {
        pub fn get_count(&self) -> u32 {
            self.count.lock().unwrap().get()
        }
    }

    // increment 1 when cloned
    impl Clone for Counter {
        fn clone(&self) -> Self {
            let mut lk = self.count.lock().unwrap();
            *lk.get_mut() += 1;
            Self {
                count: self.count.clone(),
            }
        }
    }

    // decrement 1 when dropped
    impl Drop for Counter {
        fn drop(&mut self) {
            let mut lk = self.count.lock().unwrap();
            *lk.get_mut() -= 1;
        }
    }

    #[test]
    fn counter_test() {
        let c = Counter::default();
        assert_eq!(c.get_count(), 1);
        let c2 = c.clone();
        assert_eq!(c.get_count(), 2);
        assert_eq!(c2.get_count(), 2);
        std::mem::drop(c2);
        assert_eq!(c.get_count(), 1);
    }

    #[test]
    fn com_ref_count_test() {
        let c = Counter::default();
        let my_str = CountingString::new(HSTRING::from("mydata"), c.clone());
        assert_eq!(c.get_count(), 2);
        // convert com from impl should not decrement ref.
        let com: IFabricStringResult = my_str.into();
        assert_eq!(c.get_count(), 2);
        // clone com should not change counter.
        let com2 = com.clone();
        assert_eq!(c.get_count(), 2);
        // drop com objects
        std::mem::drop(com2);
        std::mem::drop(com);
        assert_eq!(c.get_count(), 1);
    }

    #[tokio::test]
    async fn com_ref_count_multi_thread_test() {
        let h = tokio::runtime::Handle::current();
        let c = Counter::default();
        let my_str = CountingString::new(HSTRING::from("mydata"), c.clone());
        // let com1 : IFabricStringResult = my_str.into();
        // let com2 = com1.clone();
        // let h = std::thread::spawn(move||{
        //     std::thread::sleep(Duration::from_millis(10));
        //     let s: &CountingString = unsafe { com2.as_impl() };
        //     assert_eq!(s.counter.get_count(),2);
        //     com2
        // });
        // std::mem::drop(com1);
        // assert_eq!(c.get_count(), 2);
        // let com22 = h.join().unwrap();
        // assert_eq!(c.get_count(), 2);
        // std::mem::drop(com22);
        // assert_eq!(c.get_count(), 1);
        let com = my_str.my_action(h);
        assert_eq!(c.get_count(), 2);
        std::mem::drop(com);
        assert_eq!(c.get_count(), 2);
        tokio::time::sleep(Duration::from_secs(1)).await;
        assert_eq!(c.get_count(), 1);
    }
}

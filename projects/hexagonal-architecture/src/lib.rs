pub mod app;

pub mod ui_types;

#[cfg(feature = "ssr")]
pub mod config;
#[cfg(feature = "ssr")]
pub mod middleware;
#[cfg(feature = "ssr")]
pub mod server_types;
#[cfg(feature = "ssr")]
pub mod trait_impl;
#[cfg(feature = "ssr")]
pub mod traits;

#[cfg(feature = "hydrate")]
#[wasm_bindgen::prelude::wasm_bindgen]
pub fn hydrate() {
    use crate::app::*;
    console_error_panic_hook::set_once();
    leptos::mount::hydrate_body(App);
}

#[cfg(test)]
pub mod tests {
    use super::server_types::*;
    use super::traits::*;
    use std::error::Error;

    #[tokio::test]
    pub async fn test_subdomain_1_with_mocks() -> Result<(), Box<dyn Error>> {
        let mut mock_external_service_1 = MockExternalServiceTrait1::new();
        mock_external_service_1
            .expect_external_service_1_method()
            .returning(|| {
                println!("Mock external service 1");
                Ok(ExternalService1Data)
            });
        let mut mock_external_service_2 = MockExternalServiceTrait2::new();
        mock_external_service_2
            .expect_external_service_2_method()
            .returning(|| {
                println!("Mock external service 2");
                Ok(ExternalService2Data)
            });
        let real_subdomain_1_with_mock_externals = SubDomainStruct1 {
            external_service_1: mock_external_service_1,
            external_service_2: mock_external_service_2,
        };
        let data = real_subdomain_1_with_mock_externals
            .sub_domain_1_method()
            .await?;
        assert_eq!(data, SubDomain1Data);
        Ok(())
    }

    #[tokio::test]
    pub async fn test_subdomain_2_with_mocks() -> Result<(), Box<dyn Error>> {
        let mut mock_external_service_1 = MockExternalServiceTrait1::new();
        mock_external_service_1
            .expect_external_service_1_method()
            .returning(|| {
                println!("Mock external service 1 AGAIN");
                Ok(ExternalService1Data)
            });
        let real_subdomain_2_with_mock_externals = SubDomainStruct2 {
            external_service_1: mock_external_service_1,
        };
        let data = real_subdomain_2_with_mock_externals
            .sub_domain_2_method()
            .await?;
        assert_eq!(data, SubDomain2Data);
        Ok(())
    }

    #[tokio::test]
    pub async fn test_handler_with_mocks() -> Result<(), Box<dyn Error>> {
        let mut mock_subdomain_1_trait = MockSubDomainTrait1::new();
        mock_subdomain_1_trait
            .expect_sub_domain_1_method()
            .returning(|| {
                println!("Mock Subdomain 1");
                Ok(SubDomain1Data)
            });
        let mut mock_subdomain_2_trait = MockSubDomainTrait2::new();
        mock_subdomain_2_trait
            .expect_sub_domain_2_method()
            .returning(|| {
                println!("Mock Subdomain 2");
                Ok(SubDomain2Data)
            });
        let real_handler_with_mock_subdomains = HandlerStruct {
            sub_domain_1: mock_subdomain_1_trait,
            sub_domain_2: mock_subdomain_2_trait,
        };
        let data = real_handler_with_mock_subdomains.server_fn_1().await?;
        assert_eq!(data, DomainData);
        let data = real_handler_with_mock_subdomains.server_fn_2().await?;
        assert_eq!(data, DomainData2);
        let data = real_handler_with_mock_subdomains.server_fn_3().await?;
        assert_eq!(data, DomainData3);
        Ok(())
    }

    fn mock_subdomain_1() -> SubDomainStruct1<MockExternalServiceTrait1, MockExternalServiceTrait2>
    {
        let mut mock_external_service_1 = MockExternalServiceTrait1::new();
        mock_external_service_1
            .expect_external_service_1_method()
            .returning(|| {
                println!("Mock external service 1");
                Ok(ExternalService1Data)
            });
        let mut mock_external_service_2 = MockExternalServiceTrait2::new();
        mock_external_service_2
            .expect_external_service_2_method()
            .returning(|| {
                println!("Mock external service 2");
                Ok(ExternalService2Data)
            });
        let real_subdomain_1_with_mock_externals = SubDomainStruct1 {
            external_service_1: mock_external_service_1,
            external_service_2: mock_external_service_2,
        };
        real_subdomain_1_with_mock_externals
    }

    #[tokio::test]
    pub async fn test_handler_with_mock_and_real_mix() -> Result<(), Box<dyn Error>> {
        let sub_domain_1 = mock_subdomain_1();
        let mut mock_subdomain_2_trait = MockSubDomainTrait2::new();
        mock_subdomain_2_trait
            .expect_sub_domain_2_method()
            .returning(|| {
                println!("Mock Subdomain 2");
                Ok(SubDomain2Data)
            });
        let real_handler = HandlerStruct {
            sub_domain_1,
            sub_domain_2: mock_subdomain_2_trait,
        };
        let data = real_handler.server_fn_1().await?;
        assert_eq!(data, DomainData);
        let data = real_handler.server_fn_2().await?;
        assert_eq!(data, DomainData2);
        let data = real_handler.server_fn_3().await?;
        assert_eq!(data, DomainData3);
        Ok(())
    }
}

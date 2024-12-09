use crate::ui_types::*;

use super::server_types::*;
use super::traits::*;
use axum::async_trait;
use axum::extract::FromRef;
use leptos::config::LeptosOptions;

// So we can pass our server state as state into our leptos router.
impl<Handler: HandlerTrait + Clone> FromRef<ServerState<Handler>> for LeptosOptions {
    fn from_ref(input: &ServerState<Handler>) -> Self {
        input.leptos_options.clone()
    }
}

#[async_trait]
impl<SubDomain1, SubDomain2> HandlerTrait for HandlerStruct<SubDomain1, SubDomain2>
where
    SubDomain1: SubDomainTrait1 + Send + Sync,
    SubDomain2: SubDomainTrait2 + Send + Sync,
{
    async fn server_fn_1(&self) -> Result<DomainData, DomainError> {
        Ok(self.sub_domain_1.sub_domain_1_method().await?.into())
    }

    async fn server_fn_2(&self) -> Result<DomainData2, DomainError> {
        Ok(self.sub_domain_2.sub_domain_2_method().await?.into())
    }

    async fn server_fn_3(&self) -> Result<DomainData3, DomainError> {
        Ok((
            self.sub_domain_1.sub_domain_1_method().await?,
            self.sub_domain_2.sub_domain_2_method().await?,
        )
            .into())
    }
}

#[async_trait]
impl<ExternalService1, ExternalService2> SubDomainTrait1
    for SubDomainStruct1<ExternalService1, ExternalService2>
where
    ExternalService1: ExternalServiceTrait1 + Send + Sync,
    ExternalService2: ExternalServiceTrait2 + Send + Sync,
{
    async fn sub_domain_1_method(&self) -> Result<SubDomain1Data, SubDomain1Error> {
        Ok((
            self.external_service_1.external_service_1_method().await?,
            self.external_service_2.external_service_2_method().await?,
        )
            .into())
    }
}

#[async_trait]
impl<ExternalService1> SubDomainTrait2 for SubDomainStruct2<ExternalService1>
where
    ExternalService1: ExternalServiceTrait1 + Send + Sync,
{
    async fn sub_domain_2_method(&self) -> Result<SubDomain2Data, SubDomain2Error> {
        Ok(self
            .external_service_1
            .external_service_1_method()
            .await?
            .into())
    }
}

#[async_trait]
impl ExternalServiceTrait1 for ExternalService1_1 {
    async fn external_service_1_method(
        &self,
    ) -> Result<ExternalService1Data, ExternalService1Error> {
        println!("External Service 1 From External Service 1_1");
        Ok(ExternalService1Data)
    }
}
#[async_trait]
impl ExternalServiceTrait1 for ExternalService1_2 {
    async fn external_service_1_method(
        &self,
    ) -> Result<ExternalService1Data, ExternalService1Error> {
        println!("External Service 1 From External Service 1_2");
        Ok(ExternalService1Data)
    }
}
#[async_trait]
impl ExternalServiceTrait2 for ExternalService2_1 {
    async fn external_service_2_method(
        &self,
    ) -> Result<ExternalService2Data, ExternalService2Error> {
        println!("External Service 2 From External Service 2_1");
        Ok(ExternalService2Data)
    }
}
#[async_trait]
impl ExternalServiceTrait2 for ExternalService2_2 {
    async fn external_service_2_method(
        &self,
    ) -> Result<ExternalService2Data, ExternalService2Error> {
        println!("External Service 2 From External Service 2_2");
        Ok(ExternalService2Data)
    }
}

// Sub Domain mapping
impl From<(ExternalService1Data, ExternalService2Data)> for SubDomain1Data {
    fn from(_: (ExternalService1Data, ExternalService2Data)) -> Self {
        Self
    }
}
impl From<ExternalService1Data> for SubDomain2Data {
    fn from(_: ExternalService1Data) -> Self {
        Self
    }
}
// Domain Mapping
impl From<SubDomain1Data> for DomainData {
    fn from(_: SubDomain1Data) -> Self {
        Self
    }
}
impl From<SubDomain2Data> for DomainData2 {
    fn from(_: SubDomain2Data) -> Self {
        Self
    }
}
impl From<(SubDomain1Data, SubDomain2Data)> for DomainData3 {
    fn from(_: (SubDomain1Data, SubDomain2Data)) -> Self {
        Self
    }
}

// Ui Mapping
impl From<DomainData> for UiMappingFromDomainData {
    fn from(_: DomainData) -> Self {
        Self
    }
}
impl From<DomainData2> for UiMappingFromDomainData2 {
    fn from(_: DomainData2) -> Self {
        Self
    }
}
impl From<DomainData3> for UiMappingFromDomainData3 {
    fn from(_: DomainData3) -> Self {
        Self
    }
}

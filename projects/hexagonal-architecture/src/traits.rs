use super::server_types::*;
use axum::async_trait;
use mockall::automock;
pub trait New {
    fn new() -> Self;
}

#[automock]
#[async_trait]
pub trait HandlerTrait {
    async fn server_fn_1(&self) -> Result<DomainData, DomainError>;
    async fn server_fn_2(&self) -> Result<DomainData2, DomainError>;
    async fn server_fn_3(&self) -> Result<DomainData3, DomainError>;
}

#[automock]
#[async_trait]
pub trait SubDomainTrait1 {
    async fn sub_domain_1_method(&self) -> Result<SubDomain1Data, SubDomain1Error>;
}

#[automock]
#[async_trait]
pub trait SubDomainTrait2 {
    async fn sub_domain_2_method(&self) -> Result<SubDomain2Data, SubDomain2Error>;
}

#[automock]
#[async_trait]
pub trait ExternalServiceTrait1 {
    async fn external_service_1_method(
        &self,
    ) -> Result<ExternalService1Data, ExternalService1Error>;
}

#[automock]
#[async_trait]
pub trait ExternalServiceTrait2 {
    async fn external_service_2_method(
        &self,
    ) -> Result<ExternalService2Data, ExternalService2Error>;
}

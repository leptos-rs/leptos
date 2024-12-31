### Leptos Hexagonal Design

This Blog Post / Github Repository is about applying principles of hexagonal design
    - Isolating Business Logic from Sub Domains
    - Decoupling design to improve flexibility and testablity
    - Applying the principles hierachically so that sub domains which talk to external services also implement also implement hexagonal architecture


There are specific constraints that guide our design decisions

- Server Functions Can't be Generic
- Boxed Traits Objects Have overhead, so we only want to use as much generic code as possible avoid Trait Objects

The way this works is we define the functionality of our program in the main domain (i.e the business problem and processes our app is trying to solve / proceduralize). We then create sub domains and external services, although they are represented the same. External services are usually the end nodes of your app's architectural graph. Our main application builds it's service layout using configuration flags.

```rust
pub fn config() -> MainAppHandlerAlias {
    cfg_if::cfg_if! {
                if #[cfg(feature="open_ai_wrapper")] {
                    fn server_handler_config_1() -> MainAppHandler<
                        AuthService<PostgresDb, Redis>,
                        AiMessageGen<PostgresDb,OpenAiWrapper>,
                        > {
                        MainAppHandler::new_with_postgres_and_redis_open_ai()
                    }
                    server_handler_config_1()
                } else {
                    fn server_handler_config_2() -> MainAppHandler<
                        AuthService<MySql, MemCache>,
                        OtherAiMessageGen<MySql,HuggingFaceWrapper>,
                        > {
                        MainAppHandler::new_with_my_sql_memcache_hugging_face()
                    }           
                    server_handler_config_2()
                }
            }
}

```

And we pass in our handler which implements a trait

```rust
pub trait HandlerServerFn {
    pub fn server_fn_1_inner(&self);
}
impl<S,S2> HandlerServerFn for MainAppHandler<S:SubDomain1Trait,S2:SubDomain2Trait> {
    pub fn server_fn_1_inner(&self) {
        // do thing
    }
}
```

in our main fn we produce our applications service graph and pass it to our leptos router.

```rust
main () {
   let leptos_options = conf.leptos_options;
    let routes = generate_route_list(crate::app::App);
    // our feature flag based config function.
    let handler = config();
    let handler_c = handler.clone();
    // we implement FromRef<ServerState> for LeptosOptions
    let server_state = ServerState {
        handler,
        leptos_options: leptos_options.clone(),
    };
    let app = Router::new()
        .leptos_routes_with_context(
            &server_state,
            routes,
            // We pass in the MainAppHandler struct as context so we can fetch it anywhere context is available on the server.
            // This includes in middleware we define on server functions (see middleware.rs)
            move || provide_context(handler_c.clone()),
            {
                let leptos_options = leptos_options.clone();
                move || shell(leptos_options.clone())
            },
        )
        .fallback(leptos_axum::file_and_error_handler::<
            ServerState<HandlerStructAlias>,
            _,
        >(shell))
        .with_state(server_state);
}
```

and then in our server functions 

```rust
#[server]
pub async fn server_fn_1() -> Result<(),ServerFnError> {
    // we type alias every variation of our services we plan on configuring. The alternative is using Box<dyn Trait> which isn't bad - just slower.
    Ok(expect_context::<MainAppHandlerAlias>().server_fn_1_inner())
}
```

And then we can mock and service trait in any combination like so

```rust
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
```


Check out the code in the repository for a working example.

Run the tests with 

` cargo test --features ssr `
and otherwise run
` cargo leptos serve `
and navigate to `127.0.0.1:3000`

here's a picture


![alt text](leptos_hexagonal_architecture.png)
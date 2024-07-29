/*
Follows 
https://cookbook.openai.com/examples/function_calling_with_an_openapi_spec
closely
*/

pub static SYSTEM_MESSAGE :&'static str = "
You are a helpful assistant.
Respond to the following prompt by using function_call and then summarize actions.
Ask for clarification if a user request is ambiguous.
";
use serde_json::Map;
use openai_dive::v1::api::Client;
use openai_dive::v1::models::Gpt4Engine;
use std::env;
use openai_dive::v1::resources::chat::{
    ChatCompletionFunction, ChatCompletionParameters, ChatCompletionTool, ChatCompletionToolType, ChatMessage,
    ChatMessageContent,Role,
};
use utoipa::openapi::schema::Array;
use serde_json::Value;
use utoipa::openapi::schema::SchemaType;
use utoipa::openapi::schema::Schema;
use utoipa::OpenApi;
use serde_json::json;
use utoipa::openapi::path::{PathItemType,Parameter};
use utoipa::openapi::Required;
use utoipa::openapi::schema::Object;
use utoipa::openapi::RefOr;
pub fn make_openapi_call_via_gpt(message:String) -> ChatCompletionParameters {
    let docs = super::api_doc::ApiDoc::openapi();
    let mut functions = vec![];
    // get each path and it's path item object
    for (path,path_item) in docs.paths.paths.iter(){
        // all our server functions are post.
        let operation = path_item.operations.get(&PathItemType::Post).expect("Expect POST op");
        // This name will be given to the OpenAI API as part of our functions
        let name = operation.operation_id.clone().expect("Each operation to have an operation id");

        // we'll use the description
        let desc = operation.description.clone().expect("Each operation to have a description, this is how GPT knows what the functiond does and it is helpful for calling it.");
        let mut required_list = vec![];
        let mut properties = serde_json::Map::new();
        if let Some(params) = operation.parameters.clone() {
            leptos::logging::log!("{params:#?}");
            for Parameter{name,description,required,schema,..} in params.into_iter() {
                if required == Required::True {
                    required_list.push(name.clone());
                }
                let description = description.unwrap_or_default();
                if let Some(RefOr::Ref(utoipa::openapi::schema::Ref{ref_location,..})) = schema {
                    let schema_name = ref_location.split('/').last().expect("Expecting last after split");
                    let RefOr::T(schema) = docs.components
                    .as_ref()
                    .expect("components")
                    .schemas
                    .get(schema_name)
                    .cloned()
                    .expect("{schema_name} to be in components as a schema") else {panic!("expecting T")};
                let mut output = Map::new();
                parse_schema_into_openapi_property(name.clone(),schema,&mut output);
                properties.insert(name,serde_json::Value::Object(output));
                } else if let Some(RefOr::T(schema)) = schema {
                    let mut output = Map::new();
                    parse_schema_into_openapi_property(name.clone(),schema,&mut output);
                    properties.insert(name.clone(),serde_json::Value::Object(output));                   
                }
                
            }
        } 
        let parameters = json!({
            "type": "object",
            "properties": properties,
            "required": required_list,
        });
        leptos::logging::log!("{parameters}");

        functions.push(
            ChatCompletionFunction {
                name,
                description: Some(desc),
                parameters,
            }
        )
    }

    ChatCompletionParameters {
        model: Gpt4Engine::Gpt41106Preview.to_string(),
        messages: vec![
            ChatMessage {
                role:Role::System,
                content: ChatMessageContent::Text(SYSTEM_MESSAGE.to_string()),
                ..Default::default()
            },
            ChatMessage {
                role:Role::User,
                content: ChatMessageContent::Text(message),
            ..Default::default()
        }],
        tools: Some(functions.into_iter().map(|function|{
            ChatCompletionTool {
                r#type: ChatCompletionToolType::Function,
                function,
            }
        }).collect::<Vec<ChatCompletionTool>>()),
        ..Default::default()
    }
}


pub fn parse_schema_into_openapi_property(
    name:String,
    schema:Schema,
output: &mut serde_json::Map::<String,serde_json::Value>) {

    let docs = super::api_doc::ApiDoc::openapi();
    match schema {
        Schema::Object(Object{
            schema_type,
            required,
            properties,
            ..
        }) => match schema_type{
            SchemaType::Object => {
                output.insert("type".to_string(),Value::String("object".to_string()));
                output.insert("required".to_string(),Value::Array(required.into_iter()
                    .map(|s|Value::String(s))
                    .collect::<Vec<Value>>()));
                    output.insert("properties".to_string(),{
                    let mut map = Map::new();
                    for (key,val) in properties
                        .into_iter()
                        .map(|(key,val)|{
                        let RefOr::T(schema) = val else {panic!("expecting t")};
                        let mut output = Map::new();
                        parse_schema_into_openapi_property(name.clone(),schema,&mut output);
                        (key,output)
                    }) {
                        map.insert(key,Value::Object(val));
                    }
                    Value::Object(map)
                });
    
            },
            SchemaType::Value => {
                panic!("not expecting Value here.");
                
            },
            SchemaType::String => {
                output.insert("type".to_string(),serde_json::Value::String("string".to_string()));
                
            },
            SchemaType::Integer => {
                output.insert("type".to_string(),serde_json::Value::String("integer".to_string()));
                
            },
            SchemaType::Number => {
                output.insert("type".to_string(),serde_json::Value::String("number".to_string()));
                
            },
            SchemaType::Boolean => {
                output.insert("type".to_string(),serde_json::Value::String("boolean".to_string()));
                
            },
            SchemaType::Array => {
                output.insert("type".to_string(),serde_json::Value::String("array".to_string()));
                
            },
            
        },
        Schema::Array(Array{schema_type,items,..}) => {
            match schema_type {
                SchemaType::Array => {
                    let mut map = Map::new();
                    if let RefOr::Ref(utoipa::openapi::schema::Ref{ref_location,..}) = *items {
                        let schema_name = ref_location.split('/').last().expect("Expecting last after split");
                        let RefOr::T(schema) = docs.components
                        .as_ref()
                        .expect("components")
                        .schemas
                        .get(schema_name)
                        .cloned()
                        .expect("{schema_name} to be in components as a schema") else {panic!("expecting T")};
                    let mut map = Map::new();
                    parse_schema_into_openapi_property(name.clone(),schema,&mut map);
                    output.insert(name.clone(),serde_json::Value::Object(map));
                    } else if let RefOr::T(schema) = *items {
                        let mut map = Map::new();
                        parse_schema_into_openapi_property(name.clone(),schema,&mut map);
                        output.insert(name,serde_json::Value::Object(map));
                    }
                },
                _ => panic!("if schema is an array, then I'm expecting schema type to be an array ")
            }
        }
        _ => panic!("I don't know how to handle this yet.")
    }
    
}
//     let docs = super::api_doc::ApiDoc::openapi();
use crate::app::AiServerCall;
pub async fn call_gpt_with_api(message:String) -> Vec<AiServerCall> {
    let api_key = std::env::var("OPENAI_API_KEY").expect("$OPENAI_API_KEY is not set");

    let client = Client::new(api_key);
    
    let completion_parameters = make_openapi_call_via_gpt(message);

    let result = client.chat().create(completion_parameters).await.unwrap();
    let message = result.choices[0].message.clone();
    let mut res = vec![];
    if let Some(tool_calls) = message.clone().tool_calls {
        for tool_call in tool_calls {
            let name = tool_call.function.name;
            let arguments = tool_call.function.arguments;
            res.push(AiServerCall{
                path:name,
                args:arguments,
            });
        }
    }
    res
}

/*
def openapi_to_functions(openapi_spec):
    functions = []

    for path, methods in openapi_spec["paths"].items():
        for method, spec_with_ref in methods.items():
            # 1. Resolve JSON references.
            spec = jsonref.replace_refs(spec_with_ref)

            # 2. Extract a name for the functions.
            function_name = spec.get("operationId")

            # 3. Extract a description and parameters.
            desc = spec.get("description") or spec.get("summary", "")

            schema = {"type": "object", "properties": {}}

            req_body = (
                spec.get("requestBody", {})
                .get("content", {})
                .get("application/json", {})
                .get("schema")
            )
            if req_body:
                schema["properties"]["requestBody"] = req_body

            params = spec.get("parameters", [])
            if params:
                param_properties = {
                    param["name"]: param["schema"]
                    for param in params
                    if "schema" in param
                }
                schema["properties"]["parameters"] = {
                    "type": "object",
                    "properties": param_properties,
                }

            functions.append(
                {"type": "function", "function": {"name": function_name, "description": desc, "parameters": schema}}
            )

    return functions */
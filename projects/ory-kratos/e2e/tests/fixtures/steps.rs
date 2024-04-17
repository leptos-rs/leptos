use crate::{AppWorld, EMAIL_ID_MAP};
use anyhow::anyhow;
use anyhow::{Ok, Result};
use chromiumoxide::cdp::browser_protocol::input::TimeSinceEpoch;
use chromiumoxide::cdp::browser_protocol::network::{CookieParam, DeleteCookiesParams};
use cucumber::{given, then, when};
use fake::locales::EN;
use fake::{faker::internet::raw::FreeEmail, Fake};

use super::wait;
#[given("I pass")]
pub async fn i_pass(_world: &mut AppWorld) -> Result<()> {
    tracing::info!("I pass and I trace.");
    Ok(())
}

#[given("I am on the homepage")]
pub async fn navigate_to_homepage(world: &mut AppWorld) -> Result<()> {
    world.goto_path("/").await?;
    Ok(())
}

#[then("I am on the homepage")]
pub async fn check_url_for_homepage(world: &mut AppWorld) -> Result<()> {
    world.verify_route("/").await?;
    Ok(())
}

#[given("I click register")]
#[when("I click register")]
pub async fn click_register(world: &mut AppWorld) -> Result<()> {
    world.click(ids::REGISTER_BUTTON_ID).await?;
    Ok(())
}

#[given("I see the registration form")]
#[when("I see the registration form")]
#[then("I see the registration form")]
pub async fn find_registration_form(world: &mut AppWorld) -> Result<()> {
    world.find(ids::REGISTRATION_FORM_ID).await?;
    Ok(())
}

#[given("I see the login form")]
#[when("I see the login form")]
#[then("I see the login form")]
pub async fn find_login_form(world: &mut AppWorld) -> Result<()> {
    world.find(ids::LOGIN_FORM_ID).await?;
    Ok(())
}

#[given("I am on the registration page")]
pub async fn navigate_to_register(world: &mut AppWorld) -> Result<()> {
    world.goto_path("/register").await?;
    Ok(())
}

#[given("I enter valid credentials")]
pub async fn fill_form_fields_with_credentials(world: &mut AppWorld) -> Result<()> {
    let email = FreeEmail(EN).fake::<String>();
    world
        .set_field(ids::EMAIL_INPUT_ID, &email)
        .await
        .expect(&format!(
            "To find element with id {} BUT ERROR : ",
            ids::EMAIL_INPUT_ID
        ));
    world.clipboard.insert("email", email);
    world
        .set_field(ids::PASSWORD_INPUT_ID, ids::PASSWORD)
        .await
        .expect(&format!(
            "To find element with id {} BUT ERROR : ",
            ids::PASSWORD_INPUT_ID
        ));
    world.submit().await?;
    world.errors().await?;
    wait().await;
    Ok(())
}

#[given("I enter valid other credentials")]
pub async fn fill_form_fields_with_other_credentials(world: &mut AppWorld) -> Result<()> {
    let email = FreeEmail(EN).fake::<String>();
    world
        .set_field(ids::EMAIL_INPUT_ID, &email)
        .await
        .expect(&format!(
            "To find element with id {} BUT ERROR : ",
            ids::EMAIL_INPUT_ID
        ));
    world.clipboard.insert("other_email", email);
    world
        .set_field(ids::PASSWORD_INPUT_ID, ids::PASSWORD)
        .await
        .expect(&format!(
            "To find element with id {} BUT ERROR : ",
            ids::PASSWORD_INPUT_ID
        ));
    world.submit().await?;
    world.errors().await?;
    wait().await;
    Ok(())
}
#[given("I re-enter other valid credentials")]
#[when("I re-enter other valid credentials")]
pub async fn fill_form_fields_with_previous_other_credentials(world: &mut AppWorld) -> Result<()> {
    let email = world
        .clipboard
        .get("other_email")
        .cloned()
        .ok_or(anyhow!("Can't find other credentials in clipboard"))?;
    world
        .set_field(ids::EMAIL_INPUT_ID, &email)
        .await
        .expect("set email field");
    world
        .set_field(ids::PASSWORD_INPUT_ID, ids::PASSWORD)
        .await
        .expect("set password field");
    world.submit().await?;
    world.errors().await?;
    Ok(())
}

#[when("I enter valid credentials")]
#[when("I re-enter valid credentials")]
#[given("I re-enter valid credentials")]
pub async fn fill_form_fields_with_previous_credentials(world: &mut AppWorld) -> Result<()> {
    let email = world.clipboard.get("email").cloned();
    let email = if let Some(email) = email {
        email
    } else {
        let email = FreeEmail(EN).fake::<String>();
        world.clipboard.insert("email", email.clone());
        email
    };
    world
        .set_field(ids::EMAIL_INPUT_ID, &email)
        .await
        .expect("set email field");
    world
        .set_field(ids::PASSWORD_INPUT_ID, ids::PASSWORD)
        .await
        .expect("set password field");
    world.submit().await?;
    world.errors().await?;
    Ok(())
}

#[then("I am on the verify email page")]
pub async fn check_url_to_be_verify_page(world: &mut AppWorld) -> Result<()> {
    world.find(ids::VERIFY_EMAIL_DIV_ID).await?;
    Ok(())
}
#[given("I check my other email for the verification link and code")]
#[when("I check my other email for the verification link and code")]
pub async fn check_email_other_for_verification_link_and_code(world: &mut AppWorld) -> Result<()> {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    // we've stored the email with the id
    // so we get the id with our email from our clipboard
    let email = world
        .clipboard
        .get("other_email")
        .ok_or(anyhow!("email not found in clipboard"))?;
    let id = EMAIL_ID_MAP
        .read()
        .await
        .get(email)
        .ok_or(anyhow!("{email} not found in EMAIL_ID_MAP"))?
        .clone();
    // then we use the id to get the message from mailcrab
    let body = reqwest::get(format!("http://127.0.0.1:1080/api/message/{}/body", id))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let (code, link) = super::extract_code_and_link(&body)?;
    world.clipboard.insert("code", code);
    world.clipboard.insert("link", link);
    Ok(())
}

#[given("I check my email for the verification link and code")]
#[when("I check my email for the verification link and code")]
pub async fn check_email_for_verification_link_and_code(world: &mut AppWorld) -> Result<()> {
    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    // we've stored the email with the id
    // so we get the id with our email from our clipboard
    let email = world
        .clipboard
        .get("email")
        .ok_or(anyhow!("email not found in clipboard"))?;
    let id = EMAIL_ID_MAP
        .read()
        .await
        .get(email)
        .ok_or(anyhow!("{email} not found in EMAIL_ID_MAP"))?
        .clone();
    // then we use the id to get the message from mailcrab
    let body = reqwest::get(format!("http://127.0.0.1:1080/api/message/{}/body", id))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let (code, link) = super::extract_code_and_link(&body)?;
    world.clipboard.insert("code", code);
    world.clipboard.insert("link", link);
    Ok(())
}

#[given("I copy the code onto the verification link page")]
#[when("I copy the code onto the verification link page")]
pub async fn copy_code_onto_verification_page(world: &mut AppWorld) -> Result<()> {
    let link = world
        .clipboard
        .get("link")
        .ok_or(anyhow!("link not found in clipboard"))?
        .clone();
    world.goto_url(&link).await?;
    let code = world
        .clipboard
        .get("code")
        .ok_or(anyhow!("link not found in clipboard"))?
        .clone();
    world
        .set_field(ids::VERFICATION_CODE_ID, code)
        .await
        .expect(&format!("Can't find {}", ids::VERFICATION_CODE_ID));
    world.submit().await?;
    world.click("continue").await?;
    wait().await;
    Ok(())
}

#[when("I click login")]
#[given("I click login")]
pub async fn click_login(world: &mut AppWorld) -> Result<()> {
    world.click(ids::LOGIN_BUTTON_ID).await?;
    wait().await;
    Ok(())
}

#[given("I click logout")]
#[when("I click logout")]
pub async fn click_logout(world: &mut AppWorld) -> Result<()> {
    world.click(ids::LOGOUT_BUTTON_ID).await?;
    wait().await;
    world.errors().await?;
    Ok(())
}

#[tracing::instrument]
#[given("I am logged out")]
#[then("I am logged out")]
pub async fn check_ory_kratos_cookie_doesnt_exist(world: &mut AppWorld) -> Result<()> {
    let cookies = world.page.get_cookies().await?;
    if !cookies
        .iter()
        .filter(|c| c.name.contains("ory_kratos_session"))
        .collect::<Vec<_>>()
        .is_empty()
    {
        tracing::error!("{cookies:#?}");
        Err(anyhow!("Ory kratos cookie exists."))
    } else {
        Ok(())
    }
}

#[then("I am logged in")]
#[given("I am logged in")]
pub async fn check_ory_kratos_cookie_exists(world: &mut AppWorld) -> Result<()> {
    if world
        .page
        .get_cookies()
        .await?
        .iter()
        .filter(|c| c.name.contains("ory_kratos_session"))
        .collect::<Vec<_>>()
        .is_empty()
    {
        Err(anyhow!("Ory kratos cookie doesn't exists."))
    } else {
        Ok(())
    }
}

#[given("I add example post")]
#[when("I add example post")]
pub async fn add_content_to_box(world: &mut AppWorld) -> Result<()> {
    let content: Vec<String> = fake::faker::lorem::en::Words(0..10).fake();
    let content = content.join(" ");
    world.clipboard.insert("content", content.clone());
    world
        .set_field(ids::POST_POST_TEXT_AREA_ID, content)
        .await?;
    world.click(ids::POST_POST_SUBMIT_ID).await?;
    Ok(())
}

#[given("I see example content posted")]
#[then("I see example content posted")]
#[when("I see example content posted")]
pub async fn see_my_content_posted(world: &mut AppWorld) -> Result<()> {
    world.click(ids::POST_SHOW_LIST_BUTTON_ID).await?;
    let content = world
        .clipboard
        .get("content")
        .cloned()
        .ok_or(anyhow!("Can't find content in clipboard"))?;
    world.errors().await?;
    let _ = world.find_text(content).await?;
    Ok(())
}

#[when("I see error")]
#[then("I see error")]
pub async fn see_err(world: &mut AppWorld) -> Result<()> {
    wait().await;
    if world.errors().await.is_ok() {
        return Err(anyhow!("Expecting an error."));
    }
    Ok(())
}

#[when("I don't see error")]
#[then("I don't see error")]
pub async fn dont_see_err(world: &mut AppWorld) -> Result<()> {
    world.errors().await?;
    Ok(())
}

#[given("I add other email as editor")]
#[when("I add other email as editor")]
pub async fn add_other_email_as_editor(world: &mut AppWorld) -> Result<()> {
    let other_email = world
        .clipboard
        .get("other_email")
        .cloned()
        .ok_or(anyhow!("Can't find other email."))?;
    world
        .set_field(ids::POST_ADD_EDITOR_INPUT_ID, other_email)
        .await?;
    world.click(ids::POST_ADD_EDITOR_SUBMIT_ID).await?;
    Ok(())
}

#[when("I logout")]
pub async fn i_logout(world: &mut AppWorld) -> Result<()> {
    world.click(ids::LOGOUT_BUTTON_ID).await?;
    world.errors().await?;
    Ok(())
}
#[when("I edit example post")]
pub async fn add_new_edit_content_to_previous(world: &mut AppWorld) -> Result<()> {
    let edit_content: Vec<String> = fake::faker::lorem::en::Words(0..10).fake();
    let edit_content = edit_content.join(" ");
    world.clipboard.insert("edit_content", edit_content.clone());
    world
        .set_field(ids::POST_EDIT_TEXT_AREA_ID, edit_content)
        .await?;
    world.click(ids::POST_EDIT_SUBMIT_ID).await?;
    Ok(())
}
#[then("I see my new content posted")]
pub async fn new_content_boom_ba_da_boom(world: &mut AppWorld) -> Result<()> {
    let content = world
        .clipboard
        .get("edit_content")
        .cloned()
        .ok_or(anyhow!("Can't find content in clipboard"))?;
    world.find_text(content).await?;
    Ok(())
}
#[then("I don't see old content")]
pub async fn dont_see_old_content_posted(world: &mut AppWorld) -> Result<()> {
    let content = world
        .clipboard
        .get("content")
        .cloned()
        .ok_or(anyhow!("Can't find content in clipboard"))?;
    if world.find_text(content).await.is_ok() {
        return Err(anyhow!("But I do see old content..."));
    }
    Ok(())
}

#[given("I click show post list")]
#[when("I click show post list")]
pub async fn i_click_show_post_list(world: &mut AppWorld) -> Result<()> {
    world.click(ids::POST_SHOW_LIST_BUTTON_ID).await?;
    Ok(())
}

#[given("I clear cookies")]
pub async fn i_clear_cookies(world: &mut AppWorld) -> Result<()> {
    let cookies = world
        .page
        .get_cookies()
        .await?
        .into_iter()
        .map(|cookie| {
            DeleteCookiesParams::from_cookie(&CookieParam {
                name: cookie.name,
                value: cookie.value,
                url: None, // Since there's no direct field for URL, it's set as None
                domain: Some(cookie.domain),
                path: Some(cookie.path),
                secure: Some(cookie.secure),
                http_only: Some(cookie.http_only),
                same_site: cookie.same_site,
                // Assuming you have a way to convert f64 expires to TimeSinceEpoch
                expires: None,
                priority: Some(cookie.priority),
                same_party: Some(cookie.same_party),
                source_scheme: Some(cookie.source_scheme),
                source_port: Some(cookie.source_port),
                partition_key: cookie.partition_key,
                // Note: `partition_key_opaque` is omitted since it doesn't have a direct mapping
            })
        })
        .collect();
    world.page.delete_cookies(cookies).await?;
    Ok(())
}

#[given("I click recover email")]
pub async fn click_recover_email(world: &mut AppWorld) -> Result<()> {
    world.click(ids::RECOVER_EMAIL_BUTTON_ID).await?;
    wait().await;
    Ok(())
}
#[given("I submit valid recovery email")]
pub async fn submit_valid_recovery_email(world: &mut AppWorld) -> Result<()> {
    let email = world
        .clipboard
        .get("email")
        .cloned()
        .ok_or(anyhow!("Expecting email in clipboard if recovering email."))?;
    world
        .set_field(ids::EMAIL_INPUT_ID, &email)
        .await
        .expect("set email field");
    world.submit().await?;
    world.errors().await?;
    Ok(())
}
#[given("I check my email for recovery link and code")]
pub async fn check_email_for_recovery_link_and_code(world: &mut AppWorld) -> Result<()> {
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    // we've stored the email with the id
    // so we get the id with our email from our clipboard
    let email = world
        .clipboard
        .get("email")
        .ok_or(anyhow!("email not found in clipboard"))?;
    let id = EMAIL_ID_MAP
        .read()
        .await
        .get(email)
        .ok_or(anyhow!("{email} not found in EMAIL_ID_MAP"))?
        .clone();
    // then we use the id to get the message from mailcrab
    let body = reqwest::get(format!("http://127.0.0.1:1080/api/message/{}/body", id))
        .await
        .unwrap()
        .text()
        .await
        .unwrap();
    let code = super::extract_code(&body)?;
    world.clipboard.insert("recovery_code", code);
    Ok(())
}

#[when("I copy the code onto the recovery link page")]
pub async fn copy_code_onto_recovery_page(world: &mut AppWorld) -> Result<()> {
    // we should figure out how to be on the right page, will this just work?

    let code = world
        .clipboard
        .get("recovery_code")
        .ok_or(anyhow!("link not found in clipboard"))?
        .clone();
    world
        .set_field(ids::VERFICATION_CODE_ID, code)
        .await
        .expect(&format!("Can't find {}", ids::VERFICATION_CODE_ID));
    world.submit().await?;
    wait().await;
    Ok(())
}

#[then("I am on the settings page")]
pub async fn im_on_settings_page(world: &mut AppWorld) -> Result<()> {
    wait().await;
    world.url_contains("/settings").await?;
    Ok(())
}

#[given("I enter recovery credentials")]
#[when("I enter recovery credentials")]
pub async fn i_enter_a_new_recovery_password(world: &mut AppWorld) -> Result<()> {
    let email = world
        .clipboard
        .get("email")
        .cloned()
        .ok_or(anyhow!("Can't find credentials in clipboard"))?;
    world
        .set_field(ids::EMAIL_INPUT_ID, &email)
        .await
        .expect("set email field");
    world
        .set_field(ids::PASSWORD_INPUT_ID, ids::RECOVERY_PASSWORD)
        .await
        .expect("set password field");
    let code = world
        .clipboard
        .get("recovery_code")
        .ok_or(anyhow!("link not found in clipboard"))?
        .clone();
    world
        .set_field(ids::VERFICATION_CODE_ID, code)
        .await
        .expect(&format!("Can't find {}", ids::VERFICATION_CODE_ID));
    world.submit().await?;
    wait().await;
    Ok(())
}

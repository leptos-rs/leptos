pub static REGISTER_BUTTON_ID: &'static str = "register_button_id";
pub static REGISTRATION_FORM_ID: &'static str = "registration_form_id";

pub static EMAIL_INPUT_ID: &'static str = "email_input_id";
pub static PASSWORD_INPUT_ID: &'static str = "password_input_id";

pub static VERIFY_EMAIL_DIV_ID: &'static str = "verify_email_div_id";
pub static VERIFICATION_FORM_ID: &'static str = "verification_form_id";

pub static LOGIN_FORM_ID: &'static str = "login_form_id";

pub static REGISTER_ROUTE: &'static str = "/register";
pub static VERIFICATION_ROUTE: &'static str = "/verification";
pub static LOGIN_ROUTE: &'static str = "/login";
pub static KRATOS_ERROR_ROUTE: &'static str = "/kratos_error";
pub static RECOVERY_ROUTE: &'static str = "/recovery";
pub static SETTINGS_ROUTE: &'static str = "/settings";

pub static ERROR_ERROR_ID: &'static str = "error_template_id";
pub static ERROR_COOKIES_ID: &'static str = "error_cookies_id";

pub static VERFICATION_CODE_ID: &'static str = "verification_code_id";

pub static KRATOS_FORM_SUBMIT_ID: &'static str = "kratos_form_submit_id";

pub static LOGOUT_BUTTON_ID: &'static str = "logout_button_id";
pub static LOGIN_BUTTON_ID: &'static str = "login_button_id";
/// This function is for use in kratos_html, it takes the name of the input node and it
/// matches it according to what we've specified in the kratos schema file. If we change the schema.
/// I.e use a phone instead of an email, the identifier id will change and break tests that expect an email.
/// i.e use oidc instead of password, as auth method... that will break tests too.
/// Which is good.
pub fn match_name_to_id(name: String) -> &'static str {
    match name.as_str() {
        "traits.email" => EMAIL_INPUT_ID,
        "identifier" => EMAIL_INPUT_ID,
        "email" => EMAIL_INPUT_ID,
        "password" => PASSWORD_INPUT_ID,
        "code" => VERFICATION_CODE_ID,
        "totp_code" => VERFICATION_CODE_ID,
        _ => "",
    }
}

pub static POST_POST_TEXT_AREA_ID: &'static str = "post_post_text_area_id";
pub static POST_POST_SUBMIT_ID: &'static str = "post_post_submit_id";
pub static POST_ADD_EDITOR_BUTTON_ID: &'static str = "post_add_editor_button_id";
pub static POST_ADD_EDITOR_INPUT_ID: &'static str = "add_editor_input_id";
pub static POST_ADD_EDITOR_SUBMIT_ID: &'static str = "post_add_editor_submit_id";
pub static POST_DELETE_ID: &'static str = "post_delete_id";
pub static POST_EDIT_TEXT_AREA_ID: &'static str = "post_edit_text_area_id";
pub static POST_EDIT_SUBMIT_ID: &'static str = "post_edit_submit_id";
pub static POST_SHOW_LIST_BUTTON_ID: &'static str = "post_show_list_button_id";

pub static CLEAR_COOKIES_BUTTON_ID: &'static str = "clear_cookies_button_id";

pub static RECOVERY_FORM_ID: &'static str = "recovery_form_id";
pub static RECOVER_EMAIL_BUTTON_ID: &'static str = "recover_email_button_id";

pub static RECOVERY_PASSWORD: &'static str = "RECOVERY_SuPeRsAfEpAsSwOrD1234!";
pub static PASSWORD: &'static str = "SuPeRsAfEpAsSwOrD1234!";

pub static SETTINGS_FORM_ID: &'static str = "settings_form_id";

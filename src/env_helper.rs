use crate::exit_with_error;
use std::env;

#[allow(dead_code)]
#[inline]
pub fn get_mandatory_env_string(env_key: &str) -> String {
    env::var_os(env_key)
        .unwrap_or_else(|| exit_with_error(&format!("Cannot find {}, exiting...", env_key)))
        .into_string()
        .unwrap_or_else(|_| exit_with_error(&format!("Invalid string in {}, exiting...", env_key)))
}

#[allow(dead_code)]
#[inline]
pub fn get_mandatory_env_int(env_key: &str) -> i64 {
    let str_result = get_mandatory_env_string(env_key);
    str_result
        .parse::<i64>()
        .unwrap_or_else(|_| exit_with_error(&format!("Invalid integer in {}, exiting...", env_key)))
}

#[allow(dead_code)]
#[inline]
pub fn get_mandatory_env_bool(env_key: &str) -> bool {
    let int_result = get_mandatory_env_int(env_key);
    int_result == 1
}

#[allow(dead_code)]
#[inline]
pub fn get_env_string(env_key: &str) -> Option<String> {
    match env::var_os(env_key) {
        Some(val_os_string) => match val_os_string.into_string() {
            Ok(val_string) => Some(val_string),
            Err(_) => None,
        },
        None => None,
    }
}

#[allow(dead_code)]
#[inline]
pub fn get_env_int(env_key: &str) -> Option<i32> {
    let string_result = get_env_string(env_key);
    match string_result {
        Some(val_string) => match val_string.parse::<i32>() {
            Ok(val_integer) => Some(val_integer),
            Err(_) => None,
        },
        None => None,
    }
}

#[allow(dead_code)]
#[inline]
pub fn get_env_bool(env_key: &str) -> bool {
    let int_result = get_env_int(env_key);
    match int_result {
        Some(val_i) => val_i == 1,
        None => false,
    }
}

#[inline]
pub fn get_executable_name() -> String {
    env::current_exe()
        .unwrap()
        .file_name()
        .unwrap()
        .to_str()
        .unwrap()
        .to_owned()
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use std::panic::catch_unwind;

    #[test]
    fn test_boolean_env_var_reading_is_successful() {
        env::set_var("RS_TEST_BOOL", "1");
        assert!(get_mandatory_env_bool("RS_TEST_BOOL"));
    }

    #[test]
    fn test_boolean_env_var_reading_fail_but_with_result() {
        assert_eq!(get_env_bool("NON_EXISTENCE_ENV_VAR"), false);
    }

    #[test]
    fn test_int_env_var_reading_fail_but_with_result() {
        assert!(get_env_int("NON_EXISTENCE_ENV_VAR").is_none());
    }

    #[test]
    fn test_int_env_var_reading_successful_with_option() {
        env::set_var("RS_TEST_INT", "666");
        let env_reading = get_env_int("RS_TEST_INT");
        assert!(env_reading.is_some());
        assert_eq!(env_reading.unwrap(), 666);
    }

    #[test]
    fn test_panic_if_environment_undefined() {
        let result = catch_unwind(|| {
            get_mandatory_env_string("NON_EXISTENCE_ENV_VAR");
        });
        assert!(result.is_err());
    }
}

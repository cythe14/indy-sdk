use serde_json;
use libc::c_char;
use messages;
use std::ptr;
use utils::httpclient;
use utils::constants::*;
use utils::cstring::CStringUtils;
use utils::error;
use utils::threadpool::spawn;
use std::thread;
use error::prelude::*;

#[derive(Deserialize, Debug, Clone)]
pub struct UpdateAgentInfo {
    id: String,
    value: String,
}

/// Provision an agent in the agency, populate configuration and wallet for this agent.
/// NOTE: for asynchronous call use vcx_agent_provision_async
///
/// #Params
/// config: configuration
///
/// #Returns
/// Configuration (wallet also populated), on error returns NULL
#[no_mangle]
pub extern fn vcx_provision_agent(config: *const c_char) -> *mut c_char {
    info!("vcx_provision_agent >>>");

    let config = match CStringUtils::c_str_to_string(config) {
        Ok(Some(val)) => val,
        _ => {
            let _res: u32 = VcxError::from_msg(VcxErrorKind::InvalidOption, "Invalid pointer has been passed").into();
            return ptr::null_mut();
        }
    };

    trace!("vcx_provision_agent(config: {})", config);

    match messages::agent_utils::connect_register_provision(&config) {
        Err(e) => {
            error!("Provision Agent Error {}.", e);
            let _res: u32 = e.into();
            return ptr::null_mut();
        }
        Ok(s) => {
            debug!("Provision Agent Successful");
            let msg = CStringUtils::string_to_cstring(s);

            msg.into_raw()
        }
    }
}

/// Provision an agent in the agency, populate configuration and wallet for this agent.
/// NOTE: for synchronous call use vcx_provision_agent
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// config: configuration
///
/// cb: Callback that provides configuration or error status
///
/// #Returns
/// Configuration (wallet also populated), on error returns NULL
#[no_mangle]
pub extern fn vcx_agent_provision_async(command_handle: u32,
                                        config: *const c_char,
                                        cb: Option<extern fn(xcommand_handle: u32, err: u32, _config: *const c_char)>) -> u32 {
    info!("vcx_agent_provision_async >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(config, VcxErrorKind::InvalidOption);

    trace!("vcx_agent_provision_async(command_handle: {}, json: {})",
           command_handle, config);

    thread::spawn(move || {
        match messages::agent_utils::connect_register_provision(&config) {
            Err(e) => {
                error!("vcx_agent_provision_async_cb(command_handle: {}, rc: {}, config: NULL", command_handle, e);
                cb(command_handle, e.into(), ptr::null_mut());
            }
            Ok(s) => {
                trace!("vcx_agent_provision_async_cb(command_handle: {}, rc: {}, config: {})",
                       command_handle, error::SUCCESS.message, s);
                let msg = CStringUtils::string_to_cstring(s);
                cb(command_handle, 0, msg.as_ptr());
            }
        }
    });

    error::SUCCESS.code_num
}

/// Update information on the agent (ie, comm method and type)
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// json: updated configuration
///
/// cb: Callback that provides configuration or error status
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_agent_update_info(command_handle: u32,
                                    json: *const c_char,
                                    cb: Option<extern fn(xcommand_handle: u32, err: u32)>) -> u32 {
    info!("vcx_agent_update_info >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(json, VcxErrorKind::InvalidOption);

    trace!("vcx_agent_update_info(command_handle: {}, json: {})",
           command_handle, json);

    let agent_info: UpdateAgentInfo = match serde_json::from_str(&json) {
        Ok(x) => x,
        Err(e) => {
            return VcxError::from_msg(VcxErrorKind::InvalidOption, format!("Cannot deserialize agent info: {}", e)).into();
        }
    };

    spawn(move || {
        match messages::agent_utils::update_agent_info(&agent_info.id, &agent_info.value) {
            Ok(x) => {
                trace!("vcx_agent_update_info_cb(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);
                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(e) => {
                error!("vcx_agent_update_info_cb(command_handle: {}, rc: {})",
                       command_handle, e);
                cb(command_handle, e.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Get ledger fees from the sovrin network
///
/// #Params
/// command_handle: command handle to map callback to user context.
///
/// cb: Callback that provides the fee structure for the sovrin network
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_ledger_get_fees(command_handle: u32,
                                  cb: Option<extern fn(xcommand_handle: u32, err: u32, fees: *const c_char)>) -> u32 {
    info!("vcx_ledger_get_fees >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    trace!("vcx_ledger_get_fees(command_handle: {})",
           command_handle);

    spawn(move || {
        match ::utils::libindy::payments::get_ledger_fees() {
            Ok(x) => {
                trace!("vcx_ledger_get_fees_cb(command_handle: {}, rc: {}, fees: {})",
                       command_handle, error::SUCCESS.message, x);

                let msg = CStringUtils::string_to_cstring(x);
                cb(command_handle, error::SUCCESS.code_num, msg.as_ptr());
            }
            Err(e) => {
                warn!("vcx_ledget_get_fees_cb(command_handle: {}, rc: {}, fees: {})",
                      command_handle, e, "null");

                cb(command_handle, e.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

#[no_mangle]
pub extern fn vcx_set_next_agency_response(message_index: u32) {
    info!("vcx_set_next_agency_response >>>");

    let message = match message_index {
        1 => CREATE_KEYS_RESPONSE.to_vec(),
        2 => UPDATE_PROFILE_RESPONSE.to_vec(),
        3 => GET_MESSAGES_RESPONSE.to_vec(),
        4 => UPDATE_CREDENTIAL_RESPONSE.to_vec(),
        5 => UPDATE_PROOF_RESPONSE.to_vec(),
        6 => CREDENTIAL_REQ_RESPONSE.to_vec(),
        7 => PROOF_RESPONSE.to_vec(),
        8 => CREDENTIAL_RESPONSE.to_vec(),
        9 => GET_MESSAGES_INVITE_ACCEPTED_RESPONSE.to_vec(),
        _ => Vec::new(),
    };

    httpclient::set_next_u8_response(message);
}

/// Retrieve messages from the specified connection
///
/// #params
///
/// command_handle: command handle to map callback to user context.
///
/// message_status: optional - query for messages with the specified status
///
/// uids: optional, comma separated - query for messages with the specified uids
///
/// cb: Callback that provides array of matching messages retrieved
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_messages_download(command_handle: u32,
                                    message_status: *const c_char,
                                    uids: *const c_char,
                                    pw_dids: *const c_char,
                                    cb: Option<extern fn(xcommand_handle: u32, err: u32, messages: *const c_char)>) -> u32 {
    info!("vcx_messages_download >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);

    let message_status = if !message_status.is_null() {
        check_useful_c_str!(message_status, VcxErrorKind::InvalidOption);
        let v: Vec<&str> = message_status.split(',').collect();
        let v = v.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        Some(v.to_owned())
    } else {
        None
    };

    let uids = if !uids.is_null() {
        check_useful_c_str!(uids, VcxErrorKind::InvalidOption);
        let v: Vec<&str> = uids.split(',').collect();
        let v = v.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        Some(v.to_owned())
    } else {
        None
    };

    let pw_dids = if !pw_dids.is_null() {
        check_useful_c_str!(pw_dids, VcxErrorKind::InvalidOption);
        let v: Vec<&str> = pw_dids.split(',').collect();
        let v = v.iter().map(|s| s.to_string()).collect::<Vec<String>>();
        Some(v.to_owned())
    } else {
        None
    };

    trace!("vcx_messages_download(command_handle: {}, message_status: {:?}, uids: {:?})",
           command_handle, message_status, uids);

    spawn(move || {
        match ::messages::get_message::download_messages(pw_dids, message_status, uids) {
            Ok(x) => {
                match serde_json::to_string(&x) {
                    Ok(x) => {
                        trace!("vcx_messages_download_cb(command_handle: {}, rc: {}, messages: {})",
                               command_handle, error::SUCCESS.message, x);

                        let msg = CStringUtils::string_to_cstring(x);
                        cb(command_handle, error::SUCCESS.code_num, msg.as_ptr());
                    }
                    Err(e) => {
                        let err = VcxError::from_msg(VcxErrorKind::InvalidJson, format!("Cannot serialize messages: {}", e));
                        warn!("vcx_messages_download_cb(command_handle: {}, rc: {}, messages: {})",
                              command_handle, err, "null");

                        cb(command_handle, err.into(), ptr::null_mut());
                    }
                };
            }
            Err(e) => {
                warn!("vcx_messages_download_cb(command_handle: {}, rc: {}, messages: {})",
                      command_handle, e, "null");

                cb(command_handle, e.into(), ptr::null_mut());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Update the status of messages from the specified connection
///
/// #params
///
/// command_handle: command handle to map callback to user context.
///
/// message_status: updated status
///
/// msg_json: messages to update: [{"pairwiseDID":"QSrw8hebcvQxiwBETmAaRs","uids":["mgrmngq"]},...]
///
/// cb: Callback that provides success or failure of request
///
/// #Returns
/// Error code as a u32
#[no_mangle]
pub extern fn vcx_messages_update_status(command_handle: u32,
                                         message_status: *const c_char,
                                         msg_json: *const c_char,
                                         cb: Option<extern fn(xcommand_handle: u32, err: u32)>) -> u32 {
    info!("vcx_messages_update_status >>>");

    check_useful_c_callback!(cb, VcxErrorKind::InvalidOption);
    check_useful_c_str!(message_status, VcxErrorKind::InvalidOption);
    check_useful_c_str!(msg_json, VcxErrorKind::InvalidOption);

    trace!("vcx_messages_set_status(command_handle: {}, message_status: {:?}, uids: {:?})",
           command_handle, message_status, msg_json);

    spawn(move || {
        match ::messages::update_message::update_agency_messages(&message_status, &msg_json) {
            Ok(_) => {
                trace!("vcx_messages_set_status_cb(command_handle: {}, rc: {})",
                       command_handle, error::SUCCESS.message);

                cb(command_handle, error::SUCCESS.code_num);
            }
            Err(e) => {
                warn!("vcx_messages_set_status_cb(command_handle: {}, rc: {})",
                      command_handle, e);

                cb(command_handle, e.into());
            }
        };

        Ok(())
    });

    error::SUCCESS.code_num
}

/// Set the pool handle before calling vcx_init_minimal
///
/// #params
///
/// handle: pool handle that libvcx should use
///
/// #Returns
/// Error code as u32
#[no_mangle]
pub extern fn vcx_pool_set_handle(handle: i32) -> i32 {
    if handle <= 0 { ::utils::libindy::pool::change_pool_handle(None); }
    else { ::utils::libindy::pool::change_pool_handle(Some(handle)); }

    handle
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;
    use std::time::Duration;
    use api::return_types_u32;
    use utils::timeout::TimeoutUtils;

    #[test]
    fn test_provision_agent() {
        init!("true");

        let json_string = r#"{"agency_url":"https://enym-eagency.pdev.evernym.com","agency_did":"Ab8TvZa3Q19VNkQVzAWVL7","agency_verkey":"5LXaR43B1aQyeh94VBP8LG1Sgvjk7aNfqiksBCSjwqbf","wallet_name":"test_provision_agent","agent_seed":null,"enterprise_seed":null,"wallet_key":"key"}"#;
        let c_json = CString::new(json_string).unwrap().into_raw();

        let result = vcx_provision_agent(c_json);
        let result = CStringUtils::c_str_to_string(result).unwrap().unwrap();

        assert!(result.len() > 0);
    }

    #[test]
    fn test_create_agent() {
        init!("true");

        let json_string = r#"{"agency_url":"https://enym-eagency.pdev.evernym.com","agency_did":"Ab8TvZa3Q19VNkQVzAWVL7","agency_verkey":"5LXaR43B1aQyeh94VBP8LG1Sgvjk7aNfqiksBCSjwqbf","wallet_name":"test_provision_agent","agent_seed":null,"enterprise_seed":null,"wallet_key":"key"}"#;
        let c_json = CString::new(json_string).unwrap().into_raw();
        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        let result = vcx_agent_provision_async(cb.command_handle, c_json, Some(cb.get_callback()));
        assert_eq!(0, result);
        let result = cb.receive(Some(Duration::from_secs(2))).unwrap();
        assert!(result.is_some());
    }

    #[test]
    fn test_create_agent_fails() {
        init!("true");

        let json_string = r#"{"agency_url":"https://enym-eagency.pdev.evernym.com","agency_did":"Ab8TvZa3Q19VNkQVzAWVL7","agency_verkey":"5LXaR43B1aQyeh94VBP8LG1Sgvjk7aNfqiksBCSjwqbf","wallet_name":"test_provision_agent","agent_seed":null,"enterprise_seed":null,"wallet_key":null}"#;
        let c_json = CString::new(json_string).unwrap().into_raw();

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        let result = vcx_agent_provision_async(cb.command_handle, c_json, Some(cb.get_callback()));
        assert_eq!(0, result);
        let result = cb.receive(Some(Duration::from_secs(2)));
        assert_eq!(result, Err(error::INVALID_CONFIGURATION.code_num));
    }

    #[test]
    fn test_create_agent_fails_for_unknown_wallet_type() {
        init!("false");

        let config = json!({
            "agency_url":"https://enym-eagency.pdev.evernym.com",
            "agency_did":"Ab8TvZa3Q19VNkQVzAWVL7",
            "agency_verkey":"5LXaR43B1aQyeh94VBP8LG1Sgvjk7aNfqiksBCSjwqbf",
            "wallet_name":"test_provision_agent",
            "wallet_key":"key",
            "wallet_type":"UNKNOWN_WALLET_TYPE"
        }).to_string();

        let c_config = CString::new(config).unwrap().into_raw();

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        let result = vcx_agent_provision_async(cb.command_handle, c_config, Some(cb.get_callback()));
        assert_eq!(0, result);
        let result = cb.receive(Some(TimeoutUtils::medium_timeout()));
        assert_eq!(result, Err(error::INVALID_WALLET_CREATION.code_num));
    }

    #[test]
    fn test_update_agent_info() {
        init!("true");

        let json_string = r#"{"id":"123","value":"value"}"#;
        let c_json = CString::new(json_string).unwrap().into_raw();

        let cb = return_types_u32::Return_U32::new().unwrap();
        let result = vcx_agent_update_info(cb.command_handle, c_json, Some(cb.get_callback()));
        cb.receive(Some(Duration::from_secs(10))).unwrap();
    }

    #[test]
    fn test_update_agent_fails() {
        init!("true");

        httpclient::set_next_u8_response(REGISTER_RESPONSE.to_vec()); //set response garbage
        let json_string = r#"{"id":"123"}"#;
        let c_json = CString::new(json_string).unwrap().into_raw();

        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_agent_update_info(cb.command_handle,
                                         c_json,
                                         Some(cb.get_callback())),
                   error::INVALID_OPTION.code_num);
    }

    #[test]
    fn test_get_ledger_fees() {
        init!("true");

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_ledger_get_fees(cb.command_handle,
                                       Some(cb.get_callback())),
                   error::SUCCESS.code_num);
    }

    #[test]
    fn test_messages_download() {
        init!("true");

        let cb = return_types_u32::Return_U32_STR::new().unwrap();
        assert_eq!(vcx_messages_download(cb.command_handle, ptr::null_mut(), ptr::null_mut(), ptr::null_mut(), Some(cb.get_callback())), error::SUCCESS.code_num);
        cb.receive(Some(Duration::from_secs(10))).unwrap();
    }

    #[test]
    fn test_messages_update_status() {
        init!("true");

        let status = CString::new("MS-103").unwrap().into_raw();
        let json = CString::new(r#"[{"pairwiseDID":"QSrw8hebcvQxiwBETmAaRs","uids":["mgrmngq"]}]"#).unwrap().into_raw();

        let cb = return_types_u32::Return_U32::new().unwrap();
        assert_eq!(vcx_messages_update_status(cb.command_handle,
                                              status,
                                              json,
                                              Some(cb.get_callback())),
                   error::SUCCESS.code_num);
        cb.receive(Some(Duration::from_secs(10))).unwrap();
    }
}


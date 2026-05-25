pub mod control_ops;
pub mod helpers;
pub mod io_ops;
pub mod pattern_ops;
pub mod port_ops;
pub mod stream_ops;
pub mod types;

#[cfg(test)]
mod tests {
    use crate::server::SerialHandler;

    #[test]
    fn verify_all_tool_schemas() {
        let tools = vec![
            ("list_ports", SerialHandler::list_ports_tool_attr()),
            ("open", SerialHandler::open_tool_attr()),
            ("close", SerialHandler::close_tool_attr()),
            ("write", SerialHandler::write_tool_attr()),
            ("read", SerialHandler::read_tool_attr()),
            ("flush", SerialHandler::flush_tool_attr()),
            ("set_dtr_rts", SerialHandler::set_dtr_rts_tool_attr()),
            ("send_break", SerialHandler::send_break_tool_attr()),
            ("subscribe", SerialHandler::subscribe_tool_attr()),
            ("unsubscribe", SerialHandler::unsubscribe_tool_attr()),
            ("wait_for", SerialHandler::wait_for_tool_attr()),
        ];

        for (name, tool) in tools {
            assert!(
                tool.output_schema.is_some(),
                "{name} must have outputSchema"
            );
            assert!(tool.title.is_some(), "{name} must have title");
        }
    }
}

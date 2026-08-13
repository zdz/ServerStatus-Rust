use stat_common::server_status::StatRequest;

#[test]
fn daily_traffic_fields_default_to_zero() {
    let request = StatRequest::default();
    assert_eq!(request.daily_network_in, 0);
    assert_eq!(request.daily_network_out, 0);
}

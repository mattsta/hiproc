def test_debug_create_command_raw_response(client):
    """
    This is a temporary test to debug the exact JSON response from the
    create command endpoint.
    """
    command_data = {
        "command_string": "debug command",
        "name": "debug",
        "namespace": "debug",
        "user": "debugger",
        "scope": "personal",
    }
    response = client.post("/commands/", json=command_data)
    print(
        f"\n--- RAW SERVER RESPONSE ---\n{response.text}\n---------------------------\n"
    )
    assert response.status_code == 200

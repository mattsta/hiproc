def test_create_command_api(client):
    response = client.post(
        "/commands/",
        json={
            "command_string": "ls -la",
            "name": "list_files",
            "namespace": "files",
            "user": "testuser",
            "cwd": "/home/user",
            "hostname": "workstation",
        },
    )
    assert response.status_code == 200
    data = response.json()
    assert data["name"] == "list_files"
    assert data["user"] == "testuser"
    assert "id" in data
    assert "created_at" in data
    assert "last_used_at" in data
    assert "use_count" in data


def test_recall_command_api_updates_stats(client):
    # First, create a command to recall
    create_response = client.post(
        "/commands/",
        json={
            "command_string": "top",
            "name": "processes",
            "namespace": "system",
            "user": "testuser",
        },
    )
    assert create_response.json()["use_count"] == 0
    assert create_response.json()["last_used_at"] is None

    # Recall it
    recall_request = {
        "name": "processes",
        "namespace": "system",
        "user": "testuser",
    }
    recall_response = client.post("/commands/recall", json=recall_request)
    assert recall_response.status_code == 200
    data = recall_response.json()
    assert data["command_string"] == "top"
    assert data["use_count"] == 1
    assert data["last_used_at"] is not None


def test_recall_command_api_not_found(client):
    recall_request = {"name": "non_existent", "namespace": "system"}
    response = client.post("/commands/recall", json=recall_request)
    assert response.status_code == 404


def test_search_commands_api(client):
    client.post(
        "/commands/",
        json={
            "command_string": "docker ps",
            "name": "ps",
            "namespace": "docker",
            "user": "u",
        },
    )
    client.post(
        "/commands/",
        json={
            "command_string": "docker-compose up",
            "name": "up",
            "namespace": "docker",
            "user": "u",
        },
    )

    response = client.get("/commands/?q=docker")
    assert response.status_code == 200
    data = response.json()
    assert len(data) == 2


def test_update_command_api(client):
    # Create a command
    create_response = client.post(
        "/commands/",
        json={
            "command_string": "initial",
            "name": "edit_me",
            "namespace": "test",
            "user": "user1",
        },
    )
    command_id = create_response.json()["id"]

    # Try to update as another user (should fail)
    fail_response = client.patch(
        f"/commands/{command_id}?user=user2",
        json={"name": "new_name", "namespace": "new_ns"},
    )
    assert fail_response.status_code == 404

    # Update as the correct user
    success_response = client.patch(
        f"/commands/{command_id}?user=user1",
        json={"name": "new_name", "namespace": "new_ns"},
    )
    assert success_response.status_code == 200
    assert success_response.json()["name"] == "new_name"

    # Verify the change
    recall_request = {"name": "new_name", "namespace": "new_ns", "user": "user1"}
    recall_response = client.post("/commands/recall", json=recall_request)
    assert recall_response.json()["command_string"] == "initial"


def test_rename_command_api(client):
    # Create a command
    create_response = client.post(
        "/commands/",
        json={
            "command_string": "initial",
            "name": "old_name",
            "namespace": "old_ns",
            "user": "user1",
        },
    )
    command_id = create_response.json()["id"]

    # Rename as the correct user
    success_response = client.patch(
        f"/commands/{command_id}?user=user1",
        json={"name": "new_name", "namespace": "new_ns"},
    )
    assert success_response.status_code == 200
    data = success_response.json()
    assert data["name"] == "new_name"
    assert data["namespace"] == "new_ns"

    # Verify the change
    recall_request = {"name": "new_name", "namespace": "new_ns", "user": "user1"}
    recall_response = client.post("/commands/recall", json=recall_request)
    assert recall_response.json()["command_string"] == "initial"

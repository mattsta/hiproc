from hiproc import crud, schemas


def test_create_command(db_session):
    command_in = schemas.CommandCreate(
        command_string="echo 'hello'",
        name="hello",
        namespace="test",
        user="testuser",
        cwd="/tmp",
        hostname="testhost",
    )
    command = crud.create_command(db_session, command_in)
    assert command.name == "hello"
    assert command.user == "testuser"
    assert command.hostname == "testhost"


def test_recall_command_user_host_cwd_match(db_session):
    # Most specific
    crud.create_command(
        db_session,
        schemas.CommandCreate(
            command_string="user_host_cwd",
            name="c",
            namespace="ns",
            user="u1",
            hostname="h1",
            cwd="/cwd1",
        ),
    )
    # Less specific
    crud.create_command(
        db_session,
        schemas.CommandCreate(
            command_string="user_host",
            name="c",
            namespace="ns",
            user="u1",
            hostname="h1",
            cwd="/cwd2",
        ),
    )
    recalled = crud.recall_command(
        db_session, name="c", namespace="ns", user="u1", hostname="h1", cwd="/cwd1"
    )
    assert recalled.command_string == "user_host_cwd"


def test_recall_command_user_host_match(db_session):
    crud.create_command(
        db_session,
        schemas.CommandCreate(
            command_string="user_host",
            name="c",
            namespace="ns",
            user="u1",
            hostname="h1",
            cwd="/cwd2",
        ),
    )
    crud.create_command(
        db_session,
        schemas.CommandCreate(
            command_string="host_cwd",
            name="c",
            namespace="ns",
            user="u2",
            hostname="h1",
            cwd="/cwd1",
        ),
    )
    recalled = crud.recall_command(
        db_session,
        name="c",
        namespace="ns",
        user="u1",
        hostname="h1",
        cwd="/non_matching_cwd",
    )
    assert recalled.command_string == "user_host"


def test_recall_command_host_cwd_match(db_session):
    crud.create_command(
        db_session,
        schemas.CommandCreate(
            command_string="host_cwd",
            name="c",
            namespace="ns",
            user="u2",
            hostname="h1",
            cwd="/cwd1",
        ),
    )
    crud.create_command(
        db_session,
        schemas.CommandCreate(
            command_string="host",
            name="c",
            namespace="ns",
            user="u2",
            hostname="h1",
            cwd="/cwd2",
        ),
    )
    recalled = crud.recall_command(
        db_session,
        name="c",
        namespace="ns",
        user="non_matching_user",
        hostname="h1",
        cwd="/cwd1",
    )
    assert recalled.command_string == "host_cwd"


def test_recall_command_host_match(db_session):
    crud.create_command(
        db_session,
        schemas.CommandCreate(
            command_string="host",
            name="c",
            namespace="ns",
            user="u2",
            hostname="h1",
            cwd="/cwd2",
        ),
    )
    crud.create_command(
        db_session,
        schemas.CommandCreate(
            command_string="global",
            name="c",
            namespace="ns",
            user="u2",
            hostname="h2",
            cwd="/cwd2",
        ),
    )
    recalled = crud.recall_command(
        db_session,
        name="c",
        namespace="ns",
        user="non_matching_user",
        hostname="h1",
        cwd="/non_matching_cwd",
    )
    assert recalled.command_string == "host"


def test_recall_command_global_fallback(db_session):
    crud.create_command(
        db_session,
        schemas.CommandCreate(
            command_string="global",
            name="c",
            namespace="ns",
            user="u2",
            hostname="h2",
            cwd="/cwd2",
        ),
    )
    recalled = crud.recall_command(
        db_session, name="c", namespace="ns", user="x", hostname="x", cwd="x"
    )
    assert recalled.command_string == "global"


def test_recall_command_not_found(db_session):
    recalled = crud.recall_command(db_session, name="non_existent", namespace="ns")
    assert recalled is None

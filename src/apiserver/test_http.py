import routes
import db_connect
from fastapi.testclient import TestClient
import fastapi
import base64
import pymongo
import os
import subprocess
import pytest
import json

CREATED_CONTAINER = None


def get_container_manager_name() -> str:
    return os.environ.get("CONTAINER_MANAGER", "docker")


def get_container_name() -> str:
    global CREATED_CONTAINER
    if CREATED_CONTAINER is not None:
        return CREATED_CONTAINER
    out = subprocess.run(
        [get_container_manager_name(), 'run', '-d', '--publish-all',  'mongo'], stdout=subprocess.PIPE)
    out.check_returncode()
    container_name = out.stdout.strip()
    CREATED_CONTAINER = container_name
    return container_name


def connect_via_container(id: int) -> pymongo.database.Database:
    container_description_out = subprocess.run([get_container_manager_name(
    ), 'inspect', get_container_name()], stdout=subprocess.PIPE)
    container_description_out.check_returncode()
    container_description = json.loads(container_description_out.stdout)
    ports_settings = container_description[0]['NetworkSettings']['Ports']
    if type(ports_settings) == type([]):
        port = ports_settings[0]['hostPort']
    else:
        port = ports_settings['27017/tcp'][0]['HostPort']
    return db_connect.db_connect_url(f"mongodb://localhost:{port}/jjs-{id}")


@pytest.fixture(scope='session', autouse=True)
def delete_all_containers():
    yield
    print(f"deleting container: {CREATED_CONTAINER}")
    if CREATED_CONTAINER is None:
        return
    subprocess.check_call(
        [get_container_manager_name(), "rm", "--force", "-v", CREATED_CONTAINER])


CONNECTION_COUNTER = iter(range(0, 2**32))


def create_test_client() -> TestClient:
    id = next(CONNECTION_COUNTER)
    if "MONGODB_CONNECTION_STRING" in os.environ:
        database_url = os.environ["MONGODB_CONNECTION_STRING"]
        conn_str = f"{database_url}/jjs-{id}"
        app = routes.create_app(lambda: db_connect.db_connect_url(conn_str))
    else:
        app = routes.create_app(lambda: connect_via_container(id))
    return TestClient(app)


def test_smoke_is_dev():
    client = create_test_client()
    res = client.get("/system/is-dev")
    assert res.status_code == 200


def test_smoke_api_version():
    client = create_test_client()
    res = client.get("/system/api-version")
    assert res.status_code == 200


ROOT_AUTH = {'Authorization': 'Bearer Dev::root'}


def test_simple_run_ops():
    client = create_test_client()
    created_run = client.post("/runs", json={
        'code': base64.b64encode(b"Hello, Compiler!").decode(),
        'contest': 'trial',
        'problem': 'A',
        'toolchain': 'g++'
    }, headers=ROOT_AUTH)
    created_run.raise_for_status()

    all_runs = client.get("/runs", headers=ROOT_AUTH)
    assert all_runs.status_code == 200
    assert [created_run.json()] == all_runs.json()

    that_run = client.get(f"/runs/{created_run.json()['id']}",
                          headers=ROOT_AUTH)
    that_run.raise_for_status()
    assert that_run.json() == created_run.json()

    run_source = client.get(f"/runs/{created_run.json()['id']}/source",
                            headers=ROOT_AUTH)
    run_source.raise_for_status()
    assert run_source.json() == base64.b64encode(b"Hello, Compiler!").decode()


def test_queue():
    client = create_test_client()
    run1 = client.post('/runs', json={
        'code': base64.b64encode(b"Run 1").decode(),
        'contest': 'trial',
        'problem': 'A',
        'toolchain': 'g++'
    }, headers=ROOT_AUTH)
    run1.raise_for_status()

    deqeued_runs_1 = client.post('/queue', params={
        'limit': 5
    }, headers=ROOT_AUTH)
    deqeued_runs_1.raise_for_status()
    assert deqeued_runs_1.json() == [run1.json()]

    deqeued_runs_2 = client.post('/queue', params={
        'limit': 2
    }, headers=ROOT_AUTH)
    deqeued_runs_2.raise_for_status()
    assert deqeued_runs_2.json() == []


class TestPatches:
    def test_patch_nonexistent_returns_404(self):
        client = create_test_client()
        res = client.patch(
            '/runs/7b3629c1-98a0-467a-be66-adc7dfe598ac', json={}, headers=ROOT_AUTH)
        assert res.status_code == 404

    def test_simple_behavior(self):
        client = create_test_client()
        create_run = client.post('/runs', json={
            'code': base64.b64encode(b"A run").decode(),
            'contest': 'trial',
            'problem': 'A',
            'toolchain': 'g++'
        }, headers=ROOT_AUTH)

        create_run.raise_for_status()
        judge_status = ('Full', 'Accepted:FULL_SOLUTION')
        patch = {
            'binary': base64.b64encode(b"Compiled run").decode(),
            'status': [judge_status]
        }

        patch_run = client.patch(
            f"/runs/{create_run.json()['id']}", json=patch, headers=ROOT_AUTH)
        patch_run.raise_for_status()

        expected_patched_run = create_run.json()
        expected_patched_run['status'][judge_status[0]] = judge_status[1]

        # TODO verify `binary` field is updated too
        # to check this, we must call /runs/<id>/binary

        assert expected_patched_run == patch_run.json()


def test_create_users():
    client = create_test_client()

    create_root = client.post('/users', json={
        'login': 'root',
        'password': '12345',
        'roles': []
    }, headers=ROOT_AUTH)
    assert create_root.status_code == 409  # root already exists

    create_user_1 = client.post('/users', json={
        'login': 'user1',
        'password': '12345',
        'roles': []
    }, headers=ROOT_AUTH)
    create_user_1.raise_for_status()  # success

    create_user_2 = client.post('/users', json={
        'login': 'user1',
        'password': '54321',
        'roles': []
    }, headers=ROOT_AUTH)
    assert create_user_2.status_code == 409  # user1 already exists


def test_wrong_password():
    client = create_test_client()

    session = client.post('/auth/simple', json={
        'login': 'root',
        'password': 'root-has-no-password'
    })
    assert session.status_code == 403  # wrong password


def test_invalid_token():
    client = create_test_client()

    runs = client.get('/runs', headers={'Authorization': 'Bearer blablabla'})
    assert runs.status_code == 403  # invalid token


def test_unprivileged_user():
    client = create_test_client()

    create_user = client.post('/users', json={
        'login': 'user2',
        'password': '12345',
        'roles': []
    }, headers=ROOT_AUTH)
    create_user.raise_for_status()  # success

    session = client.post('/auth/simple', json={
        'login': 'user2',
        'password': '12345'
    })
    session.raise_for_status()  # success
    unpriv_auth = {'Authorization': 'Bearer '+session.json()['token']}

    runs = client.get('/runs', headers=unpriv_auth)
    assert runs.status_code == 403  # no permission


def test_observer():
    client = create_test_client()

    test_run = client.post('/runs', json={
        'code': base64.b64encode(b"Hello, Compiler!").decode(),
        'contest': 'trial',
        'problem': 'A',
        'toolchain': 'g++'
    }, headers=ROOT_AUTH)
    test_run.raise_for_status()  # success

    create_user = client.post('/users', json={
        'login': 'user3',
        'password': '12345',
        'roles': ['view_runs', 'view_all_runs']
    }, headers=ROOT_AUTH)
    create_user.raise_for_status()  # success

    session = client.post('/auth/simple', json={
        'login': 'user3',
        'password': '12345'
    })
    session.raise_for_status()  # success
    unpriv_auth = {'Authorization': 'Bearer '+session.json()['token']}

    runs = client.get('/runs', headers=unpriv_auth)
    runs.raise_for_status()
    assert runs.json() == [test_run.json()]

    unpriv_run = client.post('/runs', json={
        'code': base64.b64encode(b"Hello, Compiler!").decode(),
        'contest': 'trial',
        'problem': 'A',
        'toolchain': 'g++'
    }, headers=unpriv_auth)
    assert unpriv_run.status_code == 403  # cannot submit

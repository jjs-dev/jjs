import fastapi
from pydantic import BaseModel
import os
import json
import typing
import uuid
import pymongo
import urllib.parse
import functools

app = fastapi.FastAPI()


@functools.lru_cache
def db_connect() -> pymongo.database.Database:
    db_url = os.environ["DATABASE_URL"]
    client = pymongo.MongoClient(db_url)
    db_name = urllib.parse.urlparse(db_url).path.replace('/', '')
    return client[db_name]


@app.get('/system/is-dev', response_model=bool, operation_id="isDev")
def route_is_dev():
    """
    Returns if JJS is running in development mode.

    Please note that you don't have to respect this information, but following is recommended:
    1. Display it in each page/view.
    2. Change theme.
    3. On login view, add button "login as root".
    """
    return True


class ApiVersion(BaseModel):
    major: int
    minor: int


@app.get('/system/api-version', response_model=ApiVersion, operation_id="apiVersion")
def route_api_version():
    """
    Returns API version

    Version is returned in format {major: MAJOR, minor: MINOR}.
    MAJOR component is incremented, when backwards-incompatible changes were made.
    MINOR component is incremented, when backwards-compatible changes were made.

    It means, that if you tested application with apiVersion == X.Y, your application
    should assert that MAJOR = X and MINOR >= Y
    """
    return ApiVersion(major=0, minor=0)


class Run(BaseModel):
    id: uuid.UUID
    """
    Run identifier, unique in the system.
    """
    toolchain_id: str
    problem_id: str
    user_id: uuid.UUID
    contest_id: str


class RunSubmitSimpleParams(BaseModel):
    code: str
    """
    Base64-encoded source text
    """
    contest: str
    """
    Contest where run is submitted
    """
    problem: str
    """
    Problem name, relative to contest
    """
    toolchain: str
    """
    Toolchain to use when judging this run
    """


@app.post('/runs', response_model=Run, operation_id="submitRun")
def route_submit(params: RunSubmitSimpleParams):
    """
    Submits new run

    This operation creates new run, with given source code, and queues it for
    judging. Created run will be returned. All fields against `id` will match
    fields of request body; `id` will be real id of this run.
    """
    run_uuid = uuid.uuid4()
    user_id = uuid.UUID('12345678123456781234567812345678')
    r = Run(id=run_uuid, toolchain_id=params.toolchain,
            problem_id=params.problem, user_id=user_id, contest_id=params.contest)
    db = db_connect()
    db.runs.insert_one(dict(r))
    return r


@app.get('/runs', response_model=typing.List[Run], operation_id='listRuns')
def route_list_runs():
    """
    Lists runs

    This operation returns all created runs
    """

    db = db_connect()
    runs = db.runs.find()
    runs = list(map(lambda x: Run(**x), runs))
    return runs


if os.environ.get("__JJS_SPEC") is not None:
    req = os.environ["__JJS_SPEC"]
    if req == "openapi":
        print(json.dumps(app.openapi()))
    else:
        raise ValueError(f"unsupported __JJS_SPEC: {req}")

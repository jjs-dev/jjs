import fastapi
import auth
import db_models
import api_models
import uuid
import typing
import base64
import pymongo
import pydantic


class RunSubmitSimpleParams(pydantic.BaseModel):
    code: str
    """
    Base64-encoded source text.
    """
    contest: str
    """
    Contest where run is submitted.
    """
    problem: str
    """
    Problem name, relative to contest.
    """
    toolchain: str
    """
    Toolchain to use when judging this run.
    """


class RunPatch(pydantic.BaseModel):
    """
    Describes updates which will be applied to run
    """
    # TODO: this should be typing.Optional[typing.List[typing.Tuple[str, str]]]
    # we do not use correct type because pydantic generates incorrect schema
    # see https://github.com/samuelcolvin/pydantic/issues/1594
    status: typing.Optional[typing.List[typing.List[str]]]
    """
    behavior: merge
    merge key: protocol kind (i.e. pair first member)
    list of statuses to append, e.g.
    ```json
    [
        ["Full", "Partial:WRONG_ANSWER"],
        ["Contestant", "Accepted:OK"]
    ]
    ```
    """
    binary: typing.Optional[str] = None
    """
    behavior: replace
    Base64-encoded build artifact.
    Can be binary, can be archive.
    """


def create_app(db_connect: typing.Callable[[], pymongo.database.Database]) -> fastapi.FastAPI:
    app = fastapi.FastAPI()

    @app.get('/system/is-dev', response_model=bool,
             operation_id="isDev")
    def route_is_dev():
        """
        Returns if JJS is running in development mode

        Please note that you don't have to respect this information, but following is recommended:
        1. Display it in each page/view.
        2. Change theme.
        3. On login view, add button "login as root".
        """
        return True

    @app.get('/system/api-version', response_model=api_models.ApiVersion,
             operation_id="apiVersion")
    def route_api_version():
        """
        Returns API version

        Version is returned in format {major: MAJOR, minor: MINOR}.
        MAJOR component is incremented, when backwards-incompatible changes were made.
        MINOR component is incremented, when backwards-compatible changes were made.

        It means, that if you tested application with apiVersion == X.Y, your application
        should assert that MAJOR = X and MINOR >= Y
        """
        return api_models.ApiVersion(major=0, minor=0)

    @app.post('/runs', response_model=api_models.Run,
              operation_id="submitRun", dependencies=fastapi.Depends(auth.authenticate))
    def route_submit(params: RunSubmitSimpleParams, db = fastapi.Depends(db_connect)):
        """
        Submits new run

        This operation creates new run, with given source code, and queues it for
        judging. Created run will be returned. All fields against `id` will match
        fields of request body; `id` will be real id of this run.
        """
        run_uuid = uuid.uuid4()
        user_id = uuid.UUID('12345678123456781234567812345678')
        doc_main = db_models.RunMainProj(id=run_uuid, toolchain_name=params.toolchain,
                                         problem_name=params.problem, user_id=user_id, contest_name=params.contest, phase=str(db_models.RunPhase.QUEUED))

        doc_source = db_models.RunSourceProj(
            source=base64.b64decode(params.code))
        doc = {**dict(doc_main), **dict(doc_source)}
        db.runs.insert_one(doc)
        return api_models.Run(id=run_uuid, toolchain_name=params.toolchain, problem_name=params.problem, user_id=user_id, contest_name=params.contest)

    @app.get('/runs', response_model=typing.List[api_models.Run],
             operation_id='listRuns')
    def route_list_runs():
        """
        Lists runs

        This operation returns all created runs
        """
        db = db_connect()

        runs = db.runs.find(
            projection=db_models.RunMainProj.FIELDS)
        runs = list(map(api_models.Run.from_db, runs))
        return runs

    @app.get('/runs/{run_id}', response_model=api_models.Run, operation_id='getRun')
    def route_get_run(run_id: uuid.UUID):
        """
        Loads run by id
        """
        db = db_connect()

        run = db.runs.find_one(projection=db_models.RunMainProj.FIELDS, filter={
            'id': run_id
        })

        if run is None:
            raise fastapi.HTTPException(404, detail='RunNotFound')
        return api_models.Run.from_db(run)

    @app.get('/runs/{run_id}/source', response_model=str, operation_id='getRunSource', responses={
        204: {
            'description': "Run source is not available"
        }
    })
    def route_get_run_source(run_id: uuid.UUID):
        """
        Returns run source as base64-encoded JSON string
        """
        db = db_connect()

        doc = db.runs.find_one(projection=['source'], filter={
            'id': run_id
        })
        if doc is None:
            raise fastapi.HTTPException(404, detail='RunNotFound')
        if doc['source'] is None:
            raise fastapi.HTTPException(204, detail='RunSourceNotAvailable')
        return base64.b64encode(doc['source'])

    @app.patch('/runs/{run_id}', response_model=api_models.Run, operation_id='patchRun')
    def route_run_patch(run_id: uuid.UUID, patch: RunPatch):
        """
        Modifies existing run

        See `RunPatch` documentation for what can be updated.
        """
        db = db_connect()

        p = {
            '$set': {
                # mongodb dislikes empty $set
                '_nonempty': 'patch'
            }
        }
        # TODO maybe we need generic merging framework?
        if patch.binary is not None:
            p['$set']['binary'] = base64.b64decode(patch.binary)
        if patch.status is not None:
            for status_to_add in patch.status:
                if len(status_to_add) != 2:
                    raise ValueError(
                        "RunPatch.status[*] must have length exactly 2")
                p['$set'][f"status.{status_to_add[0]}"] = status_to_add[1]
        updated_run = db.runs.find_one_and_update(
            {'id': run_id}, p, projection=db_models.RunMainProj.FIELDS, return_document=pymongo.ReturnDocument.AFTER)
        if updated_run is None:
            raise fastapi.HTTPException(404, 'RunNotFound')
        return updated_run

    @app.post('/queue', response_model=typing.List[api_models.Run],
              operation_id='popRunFromQueue')
    def route_pop_from_invoke_queue(limit: int):
        """
        Returns runs that should be judged

        At most `limit` runs will be returned

        These runs are immediately locked, to prevent resource wasting.
        However, this is not safe distributed lock: on timeout lock will
        be released. It means, that in some rare situations same run can be judged
        several times. All judgings except one will be ignored.
        """
        db = db_connect()

        runs = []
        for _ in range(limit):
            filter_doc = {
                'phase': str(db_models.RunPhase.QUEUED)
            }
            update_doc = {
                '$set': {
                    'phase': str(db_models.RunPhase.LOCKED)
                }
            }
            doc = db.runs.find_one_and_update(
                filter_doc, update_doc, projection=db_models.RunMainProj.FIELDS, return_document=pymongo.ReturnDocument.AFTER)
            if doc is None:
                break
            runs.append(doc)
        return runs

    return app

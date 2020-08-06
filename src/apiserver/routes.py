import fastapi
import db_models
import api_models
import auth
import uuid
import bcrypt
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


class UserCreationParams(pydantic.BaseModel):
    login: str
    "Username."
    password: str
    "User password (in plaintext)."
    roles: typing.List[str]
    "List of roles the new user should be in."


class SimpleAuthParams(pydantic.BaseModel):
    login: str
    "Username."
    password: str
    "User password (in plaintext)."


class AuthResponse(pydantic.BaseModel):
    token: str
    "Session token for the newly-created user session."


def create_app(db_connect: typing.Callable[[], pymongo.database.Database]) -> fastapi.FastAPI:
    app = fastapi.FastAPI()

    root_uuid = uuid.UUID('000000000000-0000-0000-000000000000')
    guest_uuid = uuid.UUID('000000000000-0000-0000-000000000001')
    security = fastapi.security.http.HTTPBearer()

    def get_db(db: pymongo.database.Database = fastapi.Depends(db_connect)) -> pymongo.database.Database:
        db.users.create_index('id', unique=True)
        db.users.create_index('login', unique=True)
        # ensure the two special users are actually there to avoid possible conflicts, e.g. privilege escalation by creating the not-yet-existing root account
        try:
            get_special_user(db, root_uuid, 'root', [])
        except pymongo.errors.DuplicateKeyError:
            pass
        try:
            get_special_user(db, guest_uuid, 'guest', ['guests', 'invoker'])
        except pymongo.errors.DuplicateKeyError:
            pass
        return db

    def get_special_user(db, uuid, default_username, default_roles):
        users = list(db.users.find(
            projection=db_models.User.FIELDS,
            filter={'id': uuid}
        ))
        assert len(users) <= 1
        if users:
            return db_models.User(**users[0])
        else:
            new_user = db_models.User(
                id=uuid,
                login=default_username,
                password_hash='',  # invalid
                roles=list(default_roles)
            )
            db.users.insert_one(dict(new_user))
            return new_user

    def get_session(db: pymongo.database.Database = fastapi.Depends(get_db), token: str = fastapi.Depends(security)) -> auth.Session:
        if token == None:
            return auth.Session(token='', user_id=guest_uuid, roles=['guests'])
        if token.credentials == 'Dev::root':
            return auth.Session(token=token.credentials,
                                user_id=root_uuid, roles=[])
        sessions = list(db.sessions.find(
            projection=auth.Session.FIELDS,
            filter={'token': token.credentials}
        ))
        if not sessions:
            raise fastapi.HTTPException(403, detail='Invalid session')
        return auth.Session(**sessions[0])

    def get_user(db: pymongo.database.Database = fastapi.Depends(get_db), session: auth.Session = fastapi.Depends(get_session)) -> db_models.User:
        if session.user_id == root_uuid:
            return get_special_user(db, root_uuid, 'root', [])
        elif session.user_id == guest_uuid:
            return get_special_user(db, guest_uuid, 'guest', ['guests'])
        users = list(db.users.find(
            projection=db_models.User.FIELDS,
            filter={'id': session.user_id}
        ))
        if not users:
            raise fastapi.HTTPException(
                403, detail='The user has been deleted')
        assert len(users) == 1
        return db_models.User(**users[0])

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

    @app.post('/users', operation_id="createUser", response_model=db_models.User)
    def route_create_user(params: UserCreationParams, db: pymongo.database.Database = fastapi.Depends(get_db), session: auth.Session = fastapi.Depends(get_session)):
        """
        Creates new user
        """
        session.ensure_role('create_user')
        for i in params.roles:
            session.ensure_role(i)
        id = uuid.uuid4()
        hsh = bcrypt.hashpw(params.password.encode('utf-8'), bcrypt.gensalt())
        new_user = db_models.User(id=id, login=params.login,
                                  password_hash=hsh.decode('ascii'), roles=params.roles)
        try:
            db.users.insert_one(dict(new_user))
        except pymongo.errors.DuplicateKeyError:
            raise fastapi.HTTPException(409,
                                        detail='A user already exists with this username')
        return new_user

    @app.post('/auth/simple', response_model=AuthResponse,
              operation_id="login")
    def route_login(params: SimpleAuthParams, db: pymongo.database.Database = fastapi.Depends(get_db)):
        """
        Login using login and password

        In future, other means to authn will be added.
        """
        users = list(db.users.find(
            projection=db_models.User.FIELDS,
            filter={'login': params.login}
        ))
        if not users:
            raise fastapi.HTTPException(403,
                                        detail='User `{}` does not exist'.format(params.login))
        assert len(users) == 1
        user = db_models.User(**users[0])
        try:
            is_valid = bcrypt.checkpw(params.password.encode('utf-8'),
                                      user.password_hash.encode('ascii'))
        except:
            is_valid = False
        if not is_valid:
            raise fastapi.HTTPException(403, detail='Wrong password')
        token = 'UUID::'+str(uuid.uuid4())
        session = auth.Session(token=token, user_id=user.id, roles=user.roles)
        db.sessions.insert_one(dict(session))
        return AuthResponse(token=session.token)

    @app.post('/runs', response_model=api_models.Run,
              operation_id="submitRun")
    def route_submit(params: RunSubmitSimpleParams, db: pymongo.database.Database = fastapi.Depends(get_db), session: auth.Session = fastapi.Depends(get_session)):
        """
        Submits new run

        This operation creates new run, with given source code, and queues it for
        judging. Created run will be returned. All fields against `id` will match
        fields of request body; `id` will be real id of this run.
        """

        session.ensure_role('submit')
        run_uuid = uuid.uuid4()
        user_id = session.user_id
        doc_main = db_models.RunMainProj(id=run_uuid, toolchain_name=params.toolchain,
                                         problem_name=params.problem, user_id=user_id, contest_name=params.contest, phase=str(db_models.RunPhase.QUEUED))
        doc_source = db_models.RunSourceProj(
            source=base64.b64decode(params.code))
        doc = {**dict(doc_main), **dict(doc_source)}
        db.runs.insert_one(doc)
        return api_models.Run(id=run_uuid, toolchain_name=params.toolchain,
                              problem_name=params.problem, user_id=user_id, contest_name=params.contest)

    @app.get('/runs', response_model=typing.List[api_models.Run],
             operation_id='listRuns')
    def route_list_runs(db: pymongo.database.Database = fastapi.Depends(get_db), session: auth.Session = fastapi.Depends(get_session)):
        """
        Lists runs

        This operation returns all created runs
        """

        session.ensure_role('view_runs')
        runs = db.runs.find(
            projection=db_models.RunMainProj.FIELDS,
            filter=({} if session.has_role('view_all_runs')
                    else {'user_id': session.user_id})
        )
        runs = list(map(api_models.Run.from_db, runs))
        return runs

    @app.get('/runs/{run_id}', response_model=api_models.Run, operation_id='getRun')
    def route_get_run(run_id: uuid.UUID, db: pymongo.database.Database = fastapi.Depends(get_db), session: auth.Session = fastapi.Depends(get_session)):
        """
        Loads run by id
        """

        session.ensure_role('view_runs')
        run = db.runs.find_one(projection=db_models.RunMainProj.FIELDS, filter={
            'id': run_id
        })
        if run is None:
            raise fastapi.HTTPException(404, detail='RunNotFound')
        run = api_models.Run.from_db(run)
        if run.user_id != session.user_id and not session.has_role('view_all_runs'):
            raise fastapi.HTTPException(403, detail='Permission denied')
        return run

    @app.get('/runs/{run_id}/source', response_model=str, operation_id='getRunSource', responses={
        204: {
            'description': "Run source is not available"
        }
    })
    def route_get_run_source(run_id: uuid.UUID, db: pymongo.database.Database = fastapi.Depends(get_db), session: auth.Session = fastapi.Depends(get_session)):
        """
        Returns run source as base64-encoded JSON string
        """

        session.ensure_role('view_runs')
        session.ensure_role('view_run_source')
        doc = db.runs.find_one(projection=['user_id', 'source'], filter={
            'id': run_id,
        })
        if not session.has_role('view_all_runs') and doc['user_id'] != session.user_id:
            raise fastapi.HTTPException(403, detail='Permission denied')
        if doc is None:
            raise fastapi.HTTPException(404, detail='RunNotFound')
        if doc['source'] is None:
            raise fastapi.HTTPException(204, detail='RunSourceNotAvailable')
        return base64.b64encode(doc['source'])

    @app.get('/runs/{run_id}/live', response_model=api_models.LiveStatus, operation_id='getRunLiveStatus')
    def route_run_live_status(run_id: str):
        # TODO auth
        # TODO this is stub
        return {
            'finished': True,
            'current_test': None,
            'current_score': None
        }

    @app.patch('/runs/{run_id}', response_model=api_models.Run, operation_id='patchRun')
    def route_run_patch(run_id: uuid.UUID, patch: RunPatch, db: pymongo.database.Database = fastapi.Depends(get_db), session: auth.Session = fastapi.Depends(get_session)):
        """
        Modifies existing run

        See `RunPatch` documentation for what can be updated.
        """

        session.ensure_role('invoker')
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
    def route_pop_from_invoke_queue(limit: int, db: pymongo.database.Database = fastapi.Depends(get_db), session: auth.Session = fastapi.Depends(get_session)):
        """
        Returns runs that should be judged

        At most `limit` runs will be returned

        These runs are immediately locked, to prevent resource wasting.
        However, this is not safe distributed lock: on timeout lock will
        be released. It means, that in some rare situations same run can be judged
        several times. All judgings except one will be ignored.
        """

        session.ensure_role('invoker')
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

    @app.put('/toolchains/', response_model=api_models.Toolchain, operation_id="putToolchain")
    def route_put_tooclhain(toolchain: api_models.Toolchain, db: pymongo.database.Database = fastapi.Depends(get_db)):
        # TODO docs
        # TODO auth
        db.toolchains.insert_one(
            {'name': toolchain.name, 'image': toolchain.image})
        return toolchain

    @app.get('/toolchains/{toolchain_id}', response_model=api_models.Toolchain, operation_id="getToolchain")
    def route_get_toolchain(toolchain_id: str, db: pymongo.database.Database = fastapi.Depends(get_db)):
        # TODO docs
        # TODO auth
        # TODO error handling
        doc = db.toolchains.find_one(filter={'name': toolchain_id})
        assert doc is not None
        return doc

    @app.put('/problems/{problem_id}', response_model=bool, operation_id="putProblem")
    def route_put_problem(problem_id: str, problem_manifest: bytes = fastapi.Form(...), problem_assets: str = fastapi.File(...), db: pymongo.database.Database = fastapi.Depends(get_db)):
        # TODO docs
        # TODO auth
        doc = {
            'problem-name': problem_id,
            'manifest': problem_manifest,
            'assets': base64.b64decode(problem_assets)
        }
        db.problems.insert_one(doc)
        # TODO better return value
        return True

    return app

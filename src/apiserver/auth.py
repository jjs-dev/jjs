import fastapi
import pymongo


def authenticate(role: str, db: pymongo.database.Database, auth_token: str = fastapi.Header("X-JJS-Auth")):
    result_session = db.session.find_one(auth_token)
    if result_session is None:
        raise fastapi.HTTPException(403, 'Forbidden')
    user = db.user.find_one(result_session.user_id)
    if user is None or user.role != role:
        raise fastapi.HTTPException(403, 'Forbidden')

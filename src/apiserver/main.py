import fastapi.encoders
import os
import json
import typing
import uuid
import pymongo
import urllib.parse
import functools
import base64
import routes
import db_connect

app = routes.create_app(db_connect.db_connect_via_env)


if os.environ.get("__JJS_SPEC") is not None:
    req = os.environ["__JJS_SPEC"]
    if req == "openapi":
        print(json.dumps(app.openapi()))
    else:
        raise ValueError(f"unsupported __JJS_SPEC: {req}")

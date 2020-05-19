import fastapi
from pydantic import BaseModel

app = fastapi.FastAPI()


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

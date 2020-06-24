import fastapi
import pydantic
import typing
import uuid


class Session(pydantic.BaseModel):
    token: str
    "To be passed in the `Authorization` header by the client."
    user_id: uuid.UUID
    "UUID of the session owner."
    roles: typing.List[str]
    "List of roles the user is in. Cached here to avoid looking up the user in DB."

    def is_root(self):
        return self.user_id == uuid.UUID('000000000000-0000-0000-000000000000')

    def has_role(self, g: str) -> bool:
        return self.is_root() or g in self.roles

    def ensure_role(self, g: str):
        if not self.has_role(g):
            raise fastapi.HTTPException(403,
                                        detail='You do not have role `{}`'.format(g))


Session.FIELDS = ['token', 'user_id', 'roles']

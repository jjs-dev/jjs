from __future__ import annotations
import uuid
import pydantic
import typing
import db_models


class ApiVersion(pydantic.BaseModel):
    major: int
    minor: int


class Run(pydantic.BaseModel):
    id: uuid.UUID
    """
    Run identifier, unique in the system.
    """
    toolchain_name: str
    problem_name: str
    user_id: uuid.UUID
    contest_name: str
    status: typing.Mapping[str, str] = pydantic.Field(default_factory=dict)

    @staticmethod
    def from_db(doc: db_models.RunMainProj) -> Run:
        return Run(id=doc['id'], toolchain_name=doc['toolchain_name'],
                   user_id=doc['user_id'], contest_name=doc['contest_name'], problem_name=doc['problem_name'], status=doc['status'])

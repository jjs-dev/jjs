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


class Toolchain(pydantic.BaseModel):
    id: str
    description: str
    image: str


class LiveStatus(pydantic.BaseModel):
    current_test: typing.Optional[int]
    current_score: typing.Optional[int]
    finished: bool


class Contest(pydantic.BaseModel):
    id: str
    """
    Configured by human, something readable like 'olymp-2019', or 'test-contest'
    """
    title: str
    """
    E.g. 'Berlandian Olympiad in Informatics. Finals. Day 3.'
    """


class Problem(pydantic.BaseModel):
    name: str
    """
    Problem name, e.g. a-plus-b
    """
    rel_name: str
    """
    Problem relative name (aka problem code) as contestants see. This is usually one letter, e.g. 'A' or '3F'.
    """
    title: str
    """
    Problem title as contestants see, e.g. 'Find max flow'.
    """


class SessionToken(pydantic.BaseModel):
    data: str
    "Session token for the newly-created user session."

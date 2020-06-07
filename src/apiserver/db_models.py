from bson import ObjectId
from enum import Enum
import typing
from pydantic import BaseModel, Field


class Id(ObjectId):
    @classmethod
    def __get_validators__(cls):
        yield cls.validate

    @classmethod
    def validate(cls, v):
        if not isinstance(v, ObjectId):
            raise TypeError('ObjectId required')
        return str(v)

    @classmethod
    def __modify_schema__(cls, schema):
        schema.update({
            'Title': 'Object ID',
            'type': 'string'
        })


class RunPhase(Enum):
    """
    # QUEUED
    Run enters this state when received. 
    In this state run can be returned to invoker for judging.
    # LOCKED
    Some invoker is judging this run right now.
    There are two possibilities:
    1) Judging finished, and run transitions to the FINISHED state.
    2) Timeout is hit, and run transitions to the QUEUED state. (TBD)
    # FINISHED
    Judging is finished. All files and statuses are uploaded.
    """
    QUEUED = "queued"
    LOCKED = "locked"
    FINISHED = "finished"


class RunMainProj(BaseModel):
    toolchain_name: str
    problem_name: str
    user_id: Id
    contest_name: str
    phase: str  # RunPhase
    status: typing.Mapping[str, str] = Field(default_factory=dict)

    """
    Each item is (protocol_kind, f"{status_kind}:{status_code}" as in invoker_api::Status).
    """


RunMainProj.FIELDS = ['toolchain_name',
                      'problem_name', 'user_id', 'contest_name', 'status']


class RunSourceProj(BaseModel):
    source: typing.Optional[bytes]


class RunBinaryProj(BaseModel):
    binary: typing.Optional[bytes]


class RunProtocolsProj(BaseModel):
    protocols: typing.Mapping[str, str] = Field(default_factory=dict)
    """
    Key: invoker_api::judge_log::JudgeLogKind
    Value: json-encoded invoker_api::judge_log::JudgeLog
    """

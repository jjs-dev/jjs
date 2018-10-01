@0xae50de0c3f0c9aa9;

struct PingRequest {
    data @0: Data;
}

struct PingSuccess {
    data @0: Data;
}

struct PingFail {

}

struct PingResult {
    union {
        ok @0: PingSuccess;
        err @1: PingFail;
    }
}

struct Empty {}

struct RequestBody {
    union {
        ping @0: PingRequest;
        unusedFieldBecauseUnionMustHaveAtLeastTwoMembers @1: Empty;
    }
}



struct Request {
    query @0: RequestBody;
    auth: union {
        guest @1: Empty;
        unusedFieldBecauseUnionMustHaveAtLeastTwoMembers @2: Empty;
    }
}

struct ResponseBody {
    union {
        ping @0: PingResult;
        unusedFieldBecauseUnionMustHaveAtLeastTwoMembers @1: Empty;
    }
}

struct Response {
    result @0: ResponseBody;
}
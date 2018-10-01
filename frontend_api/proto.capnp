@0xae50de0c3f0c9aa9;

struct Ping {
    data @0: Data;
}

struct Empty {}

struct Request {
    inner: union {
        ping @0: Ping;
        unusedFieldBecauseUnionMustHaveAtLeastTwoMembers @1: Empty;
    }
}
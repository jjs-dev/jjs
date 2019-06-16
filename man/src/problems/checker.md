# Writing Checkers
## Checker protocol
- `JJS_CORR` - r-handle to file with test correct answer
- `JJS_SOL` - r-handle to file with provided answer
- `JJS_TEST` - r-handle to test input
- `JJS_CHECKER_OUT` - w-handle for checker output file. It is described below
- `JJS_CHECKER_COMMENT` - w-handle for comments file. It's content will be preserved and output into judge log as is.

### Output file format
Output file consists of entries. Each entry occupies one line and has format:

`TAG: VALUE`.

Currently, following tags are supported:
- `outcome`: must meet exactly once. Corresponding value is checker outcome. 
If this entry is not present, or present more than once, or can't be parsed, `JUDGE_FAULT` status is diagnosed

Outcome list:
- `Ok`
- `WrongAnswer`
- `PresentationError`
- `CheckerLogicError`
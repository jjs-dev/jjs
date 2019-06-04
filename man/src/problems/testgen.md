# Writing test generators
Test generator (aka testgen) is a program that generates test, based on test id
They are located in `testgens` subdirectory of source problem package
## Protocol
Following environment variables will be set:
- `JJS_TEST_ID` - test id, small number
- `JJS_TEST` - writeable handle, where testgen should write generated test.

Testgen exit code will be analyzed in following way:
- 0 - test generated successfully
- (all other) - test generation failed
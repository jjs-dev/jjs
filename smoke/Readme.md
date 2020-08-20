# JJS smoke tests
These tests help you ensure that JJS cluster is operating correctly.
## Usage
This tests are always executed against operating cluster.
```bash
JJS_API=<JJS apiserver endpoint without trailing slash, e.g. http://localhost:1779>
JJS_BEARER=<auth token>
# if JJS is running in development mode, use "Dev::root"
python -m pytest .
```
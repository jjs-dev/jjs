import requests
import base64
import os
import time

sess = requests.Session()
sess.headers.update(
    {
        "Authorization": f"Bearer {os.environ['JJS_BEARER']}"
    }
)

ENDPOINT = os.environ["JJS_API"]

A_PLUS_B = """
#include <iostream>
int main() {
    long long a, b;
    std::cin >> a >> b;
    std::cout << a + b << std::endl;
}
"""


def test_basic_judging():
    created_run = sess.post(f"{ENDPOINT}/runs", json={
        'code': base64.b64encode(A_PLUS_B.encode()).decode(),
        'contest': 'trial',
        'problem': 'a-plus-b',
        'toolchain': 'gcc-cpp'
    })
    created_run.raise_for_status()
    assert created_run.json()["status"] == {}
    run_id = created_run.json()["id"]
    ok = False
    for _i in range(75):
        run_state = sess.get(f"{ENDPOINT}/runs/{run_id}")
        if run_state.json()["status"].get("full") is not None:
            assert run_state.json()["status"]["full"] == "Accepted:ACCEPTED"
            ok = True
            break
        time.sleep(1)
    assert ok

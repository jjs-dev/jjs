#include <cmath>
#include <cstdio>

int main() {
    char buf[20];
    scanf("%s", buf);
    long double res = strtold(buf, nullptr);
    res = std::sqrt(res);
    printf("%lf", (double) res);
}
# shellcheck shell=sh
# wait for jjs-frontend to start up
while ! wget http://127.0.0.1:1779/ -O - >/dev/null 2>&1
do true
done

cat > test.cpp << EOF
#include <iostream>
int main(){int64_t a,b;std::cin>>a>>b;std::cout<<a+b<<"\\n";}
EOF

busybox timeout -s SIGKILL -t 60 jjs-cli submit --contest trial --filename test.cpp --problem A --toolchain g++

{ sleep 1; poweroff; } &

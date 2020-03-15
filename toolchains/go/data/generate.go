package main

import (
    "fmt"
    "strings"
    "golang.org/x/tools/go/packages"
)

func main() {
    pkgs, err := packages.Load(nil, "std")
    if err != nil {
        panic(err)
    }
    fmt.Println("package main\nimport (")
    for _, pkg := range pkgs {
        if !strings.HasPrefix(pkg.String(), "vendor") && strings.Index(pkg.String(), "internal") == -1 {
            fmt.Println("    _ \"" + pkg.String() + "\"")
        }
    }
    fmt.Println(")\n\nfunc main() {\n}\n")
}

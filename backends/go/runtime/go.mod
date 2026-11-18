module github.com/newmassrael/sce-go-runtime

go 1.23.0

// The generated vocabulary of generated/error_reason reads the forge
// runtime's optional values (tools/codegen/trees/runtime-go.toml).
require github.com/newmassrael/sce-forge-runtime v0.0.0

replace github.com/newmassrael/sce-forge-runtime => ../forge-runtime

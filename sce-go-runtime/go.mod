module github.com/newmassrael/sce-go-runtime

go 1.22

// Procedure runtime types (ProcedureServiceRequest, ProcedureRunResult,
// RunProcedure) now live in github.com/newmassrael/sce-forge-runtime/forge;
// this module's forge/ subpackage re-exports them for backward compatibility
// with existing sce-go-runtime/forge import paths.
require github.com/newmassrael/sce-forge-runtime v0.0.0

replace github.com/newmassrael/sce-forge-runtime => ../sce-forge-runtime/go

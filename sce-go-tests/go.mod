module github.com/newmassrael/sce-go-tests

go 1.22

require (
	github.com/newmassrael/sce-go-lua v0.0.0
	github.com/newmassrael/sce-go-runtime v0.0.0
)

require github.com/Shopify/go-lua v0.0.0-20221004153744-91867de107cf // indirect

replace (
	github.com/newmassrael/sce-go-lua => ../sce-go-lua
	github.com/newmassrael/sce-go-runtime => ../sce-go-runtime
)

# Importing from Polygon

## Step 1: Download polygon package
- Go to 'Packages' section
- Request 'Standard' package generation
- Now wait several minutes, until package status is `RUNNING`
- Download generated archive
- Extract it to some location, referred as `$POLYGON_PKG`

Note: other package types ('Windows' and 'Linux', available for 'Full' packages) 
can be used for import too. However, they consume more space, and Full package are generated
slower, so Standard packages are recommended


## Step 2: Import Ppc package from polygon package
- Prepare some directory for ppc package, referred as `$PPC_PKG`
- Run: `ppc import --pkg $POLYGON_PKD --out $PPC_PKG`

## Step 3: Compile invoker package as usual
Let `$INVOKER_PKG` be target path.

Run `ppc compile --pkg $PPC_PKG --out $INVOKER_PKG`
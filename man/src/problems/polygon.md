# Importing from Polygon

## Step 1: Download polygon package
- Go to 'Packages' section
- Request 'Full' package generation
- Now wait several minutes, until package status is `RUNNING`
- Download archive of 'Linux' flavor
- Extract it to some location, referred as `$POLYGON_PKG`

## Step 2: Import Ppc package from polygon package
- Prepare some directory for ppc package, referred as `$PPC_PKG`
- Run: `ppc import --pkg $POLYGON_PKD --out $PPC_PKG`

## Step 3: Compile invoker package as usual
Let `$INVOKER_PKG` be target path.

Run `ppc compile --pkg $PPC_PKG --out $INVOKER_PKG`
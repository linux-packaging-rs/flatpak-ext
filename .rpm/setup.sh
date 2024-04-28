#!/bin/bash -x

# name of the crate/package
name=$1
# version of the crate/package
version=$2
# commit to target (latest == master)
commit=$3
# path to the spec file on the pc
path_to_spec=$4
# repo link
repo=$5

LATEST="latest"

# Clone repo and cd into it
mkdir $name-$commit && cd $name-$commit && git clone --recurse-submodules $repo .

# Get latest commit hash if commit is set to latest
if [[ "$commit" == "$LATEST" ]]
then
    commit=$(git rev-parse HEAD)
    cd .. && mv $name-latest $name-$commit && cd $name-$commit
fi

# Reset to specified commit
git reset --hard $commit

# Go back to parent directory
cd ..

# Zip source
tar -pcJf $name-$commit.tar.xz $name-$commit
rm -rf $name-$commit

# Get specfile
cp $path_to_spec $name.spec 2>/dev/null || :

# Make replacements to specfile
sed -i "/^%global ver / s/.*/%global ver $version/" $name.spec
sed -i "/^%global commit / s/.*/%global commit $commit/" $name.spec
current_date=$(date +'%Y%m%d.%H')
sed -i "/^%global date / s/.*/%global date $current_date/" $name.spec


ls -a
pwd

echo Done! $1 $2 $3 $4 $5
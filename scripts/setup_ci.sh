arch=$(uname -m)
if [[ $arch == x86_64* ]]; then
    curl -LsSf https://get.nexte.st/latest/linux | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin
# elif [[ $arch == aarch64* ]]; then
#     curl -LsSf https://get.nexte.st/latest/linux-arm | tar zxf - -C ${CARGO_HOME:-~/.cargo}/bin
fi

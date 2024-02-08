This example app demonstrates how to run Galileo map as a Rust app on Android.

To run the example install Android Studio with NDK.

Then to build the rust part:

```shell
# Install Cargo NDK
cargo install cargo-ndk

# Install android targets
rustup target add \
    aarch64-linux-android \
    armv7-linux-androideabi \
    x86_64-linux-android \
    i686-linux-android

# Export NDK location. The location and version number on your system may differ from the bellow
export ANDROID_NDK_HOME=~/Android/Sdk/ndk/26.1.10909125/

# Build the app
cargo ndk -t arm64-v8a -t armeabi-v7a -t x86 -t x86_64 -o ../app/src/main/jniLibs/ build
```

After that you can run the application from the Android Studio.
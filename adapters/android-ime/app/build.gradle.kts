plugins {
    alias(libs.plugins.android.application)
}

android {
    namespace = "com.khmerime"
    compileSdk {
        version = release(36) {
            minorApiLevel = 1
        }
    }

    defaultConfig {
        applicationId = "com.khmerime"
        minSdk = 24
        targetSdk = 36
        versionCode = 1
        versionName = "1.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
        }
        release {
            optimization {
                enable = false
            }
        }
    }
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_11
        targetCompatibility = JavaVersion.VERSION_11
    }
}

// Point JVM unit tests at the Rust dylib built by `cargo build -p khmerime_android_ime`.
// The workspace target/ lives two levels above this Gradle project root.
tasks.withType<Test> {
    val nativeDir = rootDir.resolve("../../target/debug").canonicalFile
    jvmArgs("-Djava.library.path=${nativeDir.absolutePath}")
}

dependencies {
    implementation(libs.androidx.appcompat)
    implementation(libs.androidx.core.ktx)
    implementation(libs.material)
    implementation(libs.gson)
    testImplementation(libs.junit)
    androidTestImplementation(libs.androidx.espresso.core)
    androidTestImplementation(libs.androidx.junit)
}

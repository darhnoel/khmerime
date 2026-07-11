import java.util.Properties

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
        // Play reads these, not the artifact filename. versionName is the Product
        // Version (tag-derived, e.g. 1.0.0-rc.2 — see docs/release_automation.md);
        // versionCode is the monotonic Build Number and must increase per upload.
        versionCode = System.getenv("KHMERIME_ANDROID_VERSION_CODE")?.toIntOrNull() ?: 1
        versionName = System.getenv("KHMERIME_PACKAGE_VERSION") ?: "0.0.0-dev"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    // Release signing is opt-in: drop a filled keystore.properties in this module root
    // (git-ignored — see keystore.properties.example) to sign. Without it, release stays
    // unsigned, which is fine for Google Play (Play App Signing re-signs on upload).
    signingConfigs {
        val keystoreProps = rootProject.file("keystore.properties")
        if (keystoreProps.exists()) {
            create("release") {
                val props = Properties().apply { keystoreProps.inputStream().use { load(it) } }
                storeFile = file(props.getProperty("storeFile"))
                storePassword = props.getProperty("storePassword")
                keyAlias = props.getProperty("keyAlias")
                keyPassword = props.getProperty("keyPassword")
            }
        }
    }

    buildTypes {
        debug {
            applicationIdSuffix = ".debug"
        }
        release {
            optimization {
                enable = false
            }
            signingConfig = signingConfigs.findByName("release")
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

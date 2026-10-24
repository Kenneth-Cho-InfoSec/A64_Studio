import java.io.File

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("org.jetbrains.kotlin.plugin.compose")
}

dependencyLocking {
    lockAllConfigurations()
}

android {
    namespace = "dev.a64studio"
    compileSdk = 36
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "dev.a64studio"
        minSdk = 26
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0"
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        ndk {
            abiFilters += "arm64-v8a"
        }
    }

    val releaseStore = System.getenv("A64_RELEASE_STORE_FILE")
    signingConfigs {
        if (!releaseStore.isNullOrBlank()) {
            create("release") {
                storeFile = file(releaseStore)
                storePassword = System.getenv("A64_RELEASE_STORE_PASSWORD")
                keyAlias = System.getenv("A64_RELEASE_KEY_ALIAS") ?: "a64studio"
                keyPassword = System.getenv("A64_RELEASE_KEY_PASSWORD")
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(getDefaultProguardFile("proguard-android-optimize.txt"), "proguard-rules.pro")
            signingConfig = signingConfigs.findByName("release")
        }
        debug {
            applicationIdSuffix = ".debug"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    kotlinOptions {
        jvmTarget = "17"
    }
    buildFeatures {
        compose = true
        buildConfig = true
    }
    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
    }
    lint {
        abortOnError = false
    }
}

dependencies {
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.activity:activity-compose:1.9.3")
    implementation(platform("androidx.compose:compose-bom:2024.10.01"))
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.foundation:foundation")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.8.6")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.8.6")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.8.6")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.9.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.2.1")
    androidTestImplementation("androidx.test:runner:1.6.2")
    androidTestImplementation("androidx.test:rules:1.6.1")
}

val ndkHome = System.getenv("ANDROID_NDK_HOME")
    ?: File(android.sdkDirectory, "ndk/28.2.13676358").absolutePath
val cargoBin = System.getenv("CARGO")
    ?: File(System.getProperty("user.home"), ".cargo/bin/cargo").takeIf { it.exists() }?.absolutePath
    ?: "cargo"
val rustTarget = "aarch64-linux-android"
val rustOut = rootProject.file("core/target/$rustTarget/release/liba64core.so")
val jniOut = rootProject.file("app/src/main/jniLibs/arm64-v8a/liba64core.so")

val cargoBuild by tasks.registering(Exec::class) {
    group = "build"
    description = "Cross-compiles the Rust core for arm64-v8a."
    workingDir = rootProject.file("core")
    val clang = File(ndkHome, "toolchains/llvm/prebuilt/linux-x86_64/bin/aarch64-linux-android21-clang").absolutePath
    val llvmAr = File(ndkHome, "toolchains/llvm/prebuilt/linux-x86_64/bin/llvm-ar").absolutePath
    environment("CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER", clang)
    environment("CC_aarch64_linux_android", clang)
    environment("AR_aarch64_linux_android", llvmAr)
    environment("RUSTFLAGS", "-C link-arg=-Wl,-z,max-page-size=16384")
    commandLine(cargoBin, "build", "--release", "--target", rustTarget)
    doLast {
        check(rustOut.exists()) { "Rust build did not produce ${rustOut.path}" }
        jniOut.parentFile.mkdirs()
        rustOut.copyTo(jniOut, overwrite = true)
    }
}

tasks.named("preBuild") {
    dependsOn(cargoBuild)
}

tasks.register<Delete>("cleanRust") {
    delete(rootProject.file("core/target"))
    delete(jniOut)
}

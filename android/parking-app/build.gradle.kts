plugins {
    id("com.android.application") version "8.13.2"
    id("org.jetbrains.kotlin.android") version "1.9.21"
    id("com.google.protobuf") version "0.9.4"
}

android {
    namespace = "com.sdv.parking"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.sdv.parking"
        // Target Android Automotive OS (AAOS)
        minSdk = 29  // Android 10 - minimum for AAOS
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"

        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
        debug {
            isDebuggable = true
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
        viewBinding = true
        buildConfig = true
    }

    // Source sets for generated protobuf code
    sourceSets {
        getByName("main") {
            java {
                srcDirs("build/generated/source/proto/main/java")
                srcDirs("build/generated/source/proto/main/grpc")
                srcDirs("build/generated/source/proto/main/kotlin")
            }
        }
    }

    packaging {
        resources {
            excludes += "/META-INF/{AL2.0,LGPL2.1}"
            excludes += "META-INF/INDEX.LIST"
            excludes += "META-INF/io.netty.versions.properties"
        }
    }
}

// gRPC and Protocol Buffers configuration
protobuf {
    protoc {
        artifact = "com.google.protobuf:protoc:3.25.1"
    }
    plugins {
        create("grpc") {
            artifact = "io.grpc:protoc-gen-grpc-java:1.60.0"
        }
        create("grpckt") {
            artifact = "io.grpc:protoc-gen-grpc-kotlin:1.4.1:jdk8@jar"
        }
    }
    generateProtoTasks {
        all().forEach { task ->
            task.builtins {
                create("java") {
                    option("lite")
                }
            }
            task.plugins {
                create("grpc") {
                    option("lite")
                }
                create("grpckt")
            }
        }
    }
}

dependencies {
    // Kotlin
    implementation("org.jetbrains.kotlin:kotlin-stdlib:1.9.21")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.7.3")

    // Android Core
    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.appcompat:appcompat:1.6.1")
    implementation("com.google.android.material:material:1.11.0")
    implementation("androidx.constraintlayout:constraintlayout:2.1.4")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.6.2")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.6.2")
    implementation("androidx.activity:activity-ktx:1.8.2")

    // Android Automotive OS (AAOS) specific
    implementation("androidx.car.app:app:1.4.0")
    implementation("androidx.car.app:app-projected:1.4.0")

    // gRPC for communicating with RHIVOS services
    implementation("io.grpc:grpc-okhttp:1.60.0")
    implementation("io.grpc:grpc-protobuf-lite:1.60.0")
    implementation("io.grpc:grpc-stub:1.60.0")
    implementation("io.grpc:grpc-kotlin-stub:1.4.1")

    // Protocol Buffers (lite version for Android)
    implementation("com.google.protobuf:protobuf-kotlin-lite:3.25.1")

    // TLS/Security for gRPC connections
    implementation("io.grpc:grpc-netty-shaded:1.60.0")

    // Annotation processing
    compileOnly("javax.annotation:javax.annotation-api:1.3.2")

    // Networking (for REST API calls to PARKING_FEE_SERVICE)
    implementation("com.squareup.retrofit2:retrofit:2.9.0")
    implementation("com.squareup.retrofit2:converter-gson:2.9.0")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.squareup.okhttp3:logging-interceptor:4.12.0")

    // JSON parsing
    implementation("com.google.code.gson:gson:2.10.1")

    // Logging
    implementation("com.jakewharton.timber:timber:5.0.1")

    // Testing
    testImplementation("junit:junit:4.13.2")
    testImplementation("org.jetbrains.kotlinx:kotlinx-coroutines-test:1.7.3")
    testImplementation("io.mockk:mockk:1.13.8")
    testImplementation("io.kotest:kotest-runner-junit5:5.8.0")
    testImplementation("io.kotest:kotest-assertions-core:5.8.0")
    testImplementation("io.kotest:kotest-property:5.8.0")

    // Android Testing
    androidTestImplementation("androidx.test.ext:junit:1.1.5")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.1")
    androidTestImplementation("io.grpc:grpc-testing:1.60.0")
}

// Task to copy proto files from shared proto directory
tasks.register<Copy>("copyProtos") {
    from("../../proto")
    into("src/main/proto")
    include("**/*.proto")
}

// Ensure protos are copied before generating
tasks.matching { it.name.startsWith("generate") && it.name.contains("Proto") }.configureEach {
    dependsOn("copyProtos")
}

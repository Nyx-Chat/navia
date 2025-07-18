plugins {
    id("com.android.library") version "8.11.1"
    id("org.jetbrains.kotlin.android") version "2.1.20"
    id("maven-publish")
}

android {
    namespace = "com.nyx.navia"
    compileSdk = 35

    defaultConfig {
        minSdk = 23
        
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
        consumerProguardFiles("consumer-rules.pro")
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }
    
    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
    
    kotlinOptions {
        jvmTarget = "17"
    }
    
    publishing {
        singleVariant("release") {
            withSourcesJar()
        }
    }
}

dependencies {
    // JNA for UniFFI - matching nyx-android version
    implementation("net.java.dev.jna:jna:5.13.0@aar")
    
    // Kotlin - matching nyx-android versions
    implementation("org.jetbrains.kotlin:kotlin-stdlib:2.1.20")
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
}

// Task to build Rust libraries before Android build
tasks.register<Exec>("buildRustLibraries") {
    workingDir = file("../scripts")
    commandLine("bash", "build-for-android.sh", "--release", "--package")
    
    // Make output directories exist
    doFirst {
        file("src/main/jniLibs").mkdirs()
        file("src/main/java").mkdirs()
    }
}

// Ensure Rust is built before packaging
tasks.preBuild {
    dependsOn("buildRustLibraries")
}

// Publishing configuration for GitHub Packages
afterEvaluate {
    publishing {
        publications {
            register<MavenPublication>("release") {
                groupId = "com.nyx"
                artifactId = "navia"
                version = project.findProperty("VERSION_NAME") as String? ?: "1.0.0"
                
                from(components["release"])
                
                pom {
                    name.set("Navia")
                    description.set("DIDComm library for Nyx")
                    url.set("https://github.com/Nyx-Chat/navia")
                    
                    licenses {
                        license {
                            name.set("Proprietary")
                        }
                    }
                    
                    developers {
                        developer {
                            id.set("nyx-team")
                            name.set("Nyx Development Team")
                        }
                    }
                    
                    scm {
                        connection.set("scm:git:git://github.com/Nyx-Chat/navia.git")
                        developerConnection.set("scm:git:ssh://github.com/Nyx-Chat/navia.git")
                        url.set("https://github.com/Nyx-Chat/navia")
                    }
                }
            }
        }
        
        repositories {
            maven {
                name = "GitHubPackages"
                url = uri("https://maven.pkg.github.com/Nyx-Chat/navia")
                credentials {
                    username = System.getenv("GITHUB_ACTOR") ?: project.findProperty("gpr.user") as String?
                    password = System.getenv("GITHUB_TOKEN") ?: project.findProperty("gpr.token") as String?
                }
            }
        }
    }
}
# MultiClaw Android ProGuard Rules

# Keep native bridge
-keep class ai.multiclaw.android.bridge.** { *; }
-keepclassmembers class ai.multiclaw.android.bridge.** { *; }

# Keep JNI methods
-keepclasseswithmembernames class * {
    native <methods>;
}

# Keep data classes for serialization
-keep class ai.multiclaw.android.**.data.** { *; }
-keepclassmembers class ai.multiclaw.android.**.data.** { *; }

# Kotlin serialization
-keepattributes *Annotation*, InnerClasses
-dontnote kotlinx.serialization.AnnotationsKt
-keepclassmembers class kotlinx.serialization.json.** { *** Companion; }
-keepclasseswithmembers class kotlinx.serialization.json.** { kotlinx.serialization.KSerializer serializer(...); }

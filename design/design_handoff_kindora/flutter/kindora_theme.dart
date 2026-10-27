// Kindora — Flutter ThemeData (Material 3)
// pubspec.yaml: google_fonts: ^6.2.1
//
// Kullanım:
//   MaterialApp(
//     theme: KindoraTheme.light(),
//     darkTheme: KindoraTheme.dark(),
//     themeMode: ThemeMode.system, // Ayarlar > Tema: Açık / Koyu / Sistem
//   );
//   final kd = Theme.of(context).extension<KdColors>()!;  // durum renkleri

import 'package:flutter/material.dart';
import 'package:google_fonts/google_fonts.dart';

/// Ham renk değerleri (masaüstü SCSS token'larıyla birebir aynı).
abstract final class KdPalette {
  static const primary = Color(0xFF5265D8);
  static const primaryHover = Color(0xFF4657C8);
  static const primaryActive = Color(0xFF3F50B8);
  static const secondary = Color(0xFF8C80E8);
  static const accent = Color(0xFF2BBFA7);
  static const warning = Color(0xFFE7A83E);
  static const danger = Color(0xFFD95B68);
  static const dangerSolid = Color(0xFFC9475A);

  // light neutrals
  static const bg = Color(0xFFF7F9FC);
  static const surface = Color(0xFFFFFFFF);
  static const surface2 = Color(0xFFF1F3F9);
  static const line = Color(0xFFE4E8F1);
  static const lineStrong = Color(0xFFD5DAE6);
  static const muted = Color(0xFFC9CFDD);
  static const ink = Color(0xFF1A2033);
  static const ink2 = Color(0xFF4A5370);
  static const ink3 = Color(0xFF5D6680);

  // dark neutrals
  static const dBg = Color(0xFF111827);
  static const dSurface = Color(0xFF1A2233);
  static const dSurface2 = Color(0xFF232D42);
  static const dLine = Color(0xFF2E3850);
  static const dLineStrong = Color(0xFF3A4560);
  static const dInk = Color(0xFFE9ECF5);
  static const dInk2 = Color(0xFFA9B1C7);
  static const dInk3 = Color(0xFF8D96AE);
}

/// Durum renkleri: her durum = zemin (tint) + metin + çizgi.
/// Kural: durum asla yalnızca renkle anlatılmaz — ikon + kelime + renk.
@immutable
class KdColors extends ThemeExtension<KdColors> {
  const KdColors({
    required this.primaryTint, required this.primaryText,
    required this.secondaryTint, required this.secondaryText,
    required this.successTint, required this.successText, required this.successLine,
    required this.warningTint, required this.warningText, required this.warningLine,
    required this.dangerTint, required this.dangerText, required this.dangerLine,
    required this.neutralTint, required this.infoText, required this.inverse,
    required this.ink2, required this.ink3, required this.line,
  });

  final Color primaryTint, primaryText, secondaryTint, secondaryText;
  final Color successTint, successText, successLine;
  final Color warningTint, warningText, warningLine;
  final Color dangerTint, dangerText, dangerLine;
  final Color neutralTint, infoText, inverse, ink2, ink3, line;

  static const light = KdColors(
    primaryTint: Color(0xFFEEF0FC), primaryText: Color(0xFF3F50B8),
    secondaryTint: Color(0xFFF1EFFD), secondaryText: Color(0xFF5B4FC0),
    successTint: Color(0xFFE3F7F3), successText: Color(0xFF127A6A), successLine: Color(0xFFA9E4D9),
    warningTint: Color(0xFFFDF4E3), warningText: Color(0xFF8A5A0C), warningLine: Color(0xFFF1D49C),
    dangerTint: Color(0xFFFCEBED), dangerText: Color(0xFFA8364A), dangerLine: Color(0xFFF0C2CA),
    neutralTint: Color(0xFFEEF0F5), infoText: Color(0xFF2A3580), inverse: Color(0xFF1A2033),
    ink2: KdPalette.ink2, ink3: KdPalette.ink3, line: KdPalette.line,
  );

  static const dark = KdColors(
    primaryTint: Color(0x385265D8), primaryText: Color(0xFFA9B4FF),
    secondaryTint: Color(0x298C80E8), secondaryText: Color(0xFFC3BCF6),
    successTint: Color(0x242BBFA7), successText: Color(0xFF5FDCC6), successLine: Color(0x662BBFA7),
    warningTint: Color(0x24E7A83E), warningText: Color(0xFFF3C77A), warningLine: Color(0x66E7A83E),
    dangerTint: Color(0x24D95B68), dangerText: Color(0xFFF29AA6), dangerLine: Color(0x66D95B68),
    neutralTint: Color(0xFF2A3349), infoText: Color(0xFFC9D0FF), inverse: Color(0xFF2E3850),
    ink2: KdPalette.dInk2, ink3: KdPalette.dInk3, line: KdPalette.dLine,
  );

  @override
  KdColors copyWith() => this;

  @override
  KdColors lerp(ThemeExtension<KdColors>? other, double t) {
    if (other is! KdColors) return this;
    Color l(Color a, Color b) => Color.lerp(a, b, t)!;
    return KdColors(
      primaryTint: l(primaryTint, other.primaryTint), primaryText: l(primaryText, other.primaryText),
      secondaryTint: l(secondaryTint, other.secondaryTint), secondaryText: l(secondaryText, other.secondaryText),
      successTint: l(successTint, other.successTint), successText: l(successText, other.successText), successLine: l(successLine, other.successLine),
      warningTint: l(warningTint, other.warningTint), warningText: l(warningText, other.warningText), warningLine: l(warningLine, other.warningLine),
      dangerTint: l(dangerTint, other.dangerTint), dangerText: l(dangerText, other.dangerText), dangerLine: l(dangerLine, other.dangerLine),
      neutralTint: l(neutralTint, other.neutralTint), infoText: l(infoText, other.infoText), inverse: l(inverse, other.inverse),
      ink2: l(ink2, other.ink2), ink3: l(ink3, other.ink3), line: l(line, other.line),
    );
  }
}

/// Ölçüler (dp). Mobilde dokunma alanı en az 48.
abstract final class KdSize {
  static const tap = 48.0;
  static const navBar = 80.0;
  static const radiusControl = 12.0;  // input, chip
  static const radiusCard = 16.0;     // mobil kart
  static const radiusSheet = 28.0;    // bottom sheet üst köşeler
  static const radiusFab = 16.0;
  static const space = [0.0, 4.0, 8.0, 12.0, 16.0, 20.0, 24.0, 32.0, 48.0];
}

abstract final class KdMotion {
  static const fast = Duration(milliseconds: 150);
  static const base = Duration(milliseconds: 200);
  static const sheet = Duration(milliseconds: 250);
  static const curve = Cubic(.2, .8, .2, 1);
  /// Snackbar: bilgi/başarı 4 sn; hata (SnackBarAction ile) kapatılana kadar.
  static const snackInfo = Duration(seconds: 4);
}

abstract final class KindoraTheme {
  static ThemeData light() => _build(Brightness.light);
  static ThemeData dark() => _build(Brightness.dark);

  static ThemeData _build(Brightness b) {
    final isDark = b == Brightness.dark;
    final kd = isDark ? KdColors.dark : KdColors.light;

    final scheme = ColorScheme(
      brightness: b,
      primary: KdPalette.primary,
      onPrimary: Colors.white,
      primaryContainer: kd.primaryTint,
      onPrimaryContainer: kd.primaryText,
      secondary: KdPalette.secondary,
      onSecondary: KdPalette.ink,
      secondaryContainer: kd.secondaryTint,
      onSecondaryContainer: kd.secondaryText,
      tertiary: KdPalette.accent,
      onTertiary: KdPalette.ink,
      tertiaryContainer: kd.successTint,
      onTertiaryContainer: kd.successText,
      error: KdPalette.danger,
      onError: Colors.white,
      errorContainer: kd.dangerTint,
      onErrorContainer: kd.dangerText,
      surface: isDark ? KdPalette.dSurface : KdPalette.surface,
      onSurface: isDark ? KdPalette.dInk : KdPalette.ink,
      onSurfaceVariant: isDark ? KdPalette.dInk2 : KdPalette.ink2,
      surfaceContainerLowest: isDark ? KdPalette.dBg : KdPalette.surface,
      surfaceContainerLow: isDark ? KdPalette.dSurface : KdPalette.bg,
      surfaceContainer: isDark ? KdPalette.dSurface2 : KdPalette.surface2,
      surfaceContainerHigh: isDark ? KdPalette.dSurface2 : KdPalette.surface2,
      surfaceContainerHighest: kd.neutralTint,
      outline: isDark ? KdPalette.dLineStrong : KdPalette.lineStrong,
      outlineVariant: isDark ? KdPalette.dLine : KdPalette.line,
      inverseSurface: kd.inverse,
      onInverseSurface: Colors.white,
      inversePrimary: const Color(0xFFA9B4FF),
      scrim: const Color(0x6B111827),
      shadow: const Color(0xFF1A2033),
    );

    final base = GoogleFonts.onestTextTheme();
    final ink = scheme.onSurface;
    TextStyle t(double size, FontWeight w, {double h = 1.4, Color? c}) =>
        GoogleFonts.onest(fontSize: size, fontWeight: w, height: h, color: c ?? ink);

    final text = base.copyWith(
      displaySmall: t(34, FontWeight.w600, h: 1.15),
      headlineMedium: t(26, FontWeight.w700, h: 1.2),   // büyük sayılar: "1 sa 42 dk"
      headlineSmall: t(22, FontWeight.w600, h: 1.25),   // ekran başlığı
      titleLarge: t(18, FontWeight.w600),               // sheet başlığı
      titleMedium: t(16, FontWeight.w600),              // kart başlığı
      titleSmall: t(14, FontWeight.w600),
      bodyLarge: t(15, FontWeight.w400, h: 1.5),
      bodyMedium: t(14, FontWeight.w400, h: 1.5),
      bodySmall: t(13, FontWeight.w400, h: 1.45, c: scheme.onSurfaceVariant),
      labelLarge: t(15, FontWeight.w600),               // buton
      labelMedium: t(13, FontWeight.w600),              // chip
      labelSmall: t(12, FontWeight.w500, c: kd.ink3),   // yardımcı metin (min 12)
    );

    final pill = RoundedRectangleBorder(borderRadius: BorderRadius.circular(999));
    final focusSide = const BorderSide(color: KdPalette.secondary, width: 3);

    return ThemeData(
      useMaterial3: true,
      brightness: b,
      colorScheme: scheme,
      scaffoldBackgroundColor: isDark ? KdPalette.dBg : KdPalette.bg,
      textTheme: text,
      extensions: [kd],
      visualDensity: VisualDensity.standard,
      materialTapTargetSize: MaterialTapTargetSize.padded,
      splashFactory: InkSparkle.splashFactory,

      appBarTheme: AppBarTheme(
        backgroundColor: isDark ? KdPalette.dBg : KdPalette.bg,
        foregroundColor: ink,
        elevation: 0,
        scrolledUnderElevation: 0,
        titleTextStyle: text.headlineSmall,
      ),

      cardTheme: CardThemeData(
        color: scheme.surface,
        elevation: 0,
        margin: EdgeInsets.zero,
        shape: RoundedRectangleBorder(
          borderRadius: BorderRadius.circular(KdSize.radiusCard),
          side: BorderSide(color: scheme.outlineVariant),
        ),
      ),

      filledButtonTheme: FilledButtonThemeData(
        style: FilledButton.styleFrom(
          minimumSize: const Size(64, 52),
          shape: pill,
          textStyle: text.labelLarge,
          backgroundColor: KdPalette.primary,
          foregroundColor: Colors.white,
          disabledBackgroundColor: isDark ? const Color(0xFF3A4580) : const Color(0xFFB9C2EE),
          disabledForegroundColor: Colors.white,
        ).copyWith(
          overlayColor: WidgetStatePropertyAll(KdPalette.primaryActive.withValues(alpha: .25)),
          side: WidgetStateProperty.resolveWith((s) => s.contains(WidgetState.focused) ? focusSide : null),
        ),
      ),

      outlinedButtonTheme: OutlinedButtonThemeData(
        style: OutlinedButton.styleFrom(
          minimumSize: const Size(64, 52),
          shape: pill,
          textStyle: text.labelLarge,
          foregroundColor: ink,
          side: BorderSide(color: scheme.outline),
        ),
      ),

      textButtonTheme: TextButtonThemeData(
        style: TextButton.styleFrom(
          minimumSize: const Size(48, 48),
          foregroundColor: kd.primaryText,
          textStyle: text.labelMedium,
        ),
      ),

      floatingActionButtonTheme: FloatingActionButtonThemeData(
        backgroundColor: KdPalette.primary,
        foregroundColor: Colors.white,
        extendedTextStyle: text.labelLarge,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(KdSize.radiusFab)),
      ),

      chipTheme: ChipThemeData(
        shape: pill,
        side: BorderSide(color: scheme.outlineVariant),
        selectedColor: kd.primaryTint,
        backgroundColor: scheme.surface,
        labelStyle: text.labelMedium,
        secondaryLabelStyle: text.labelMedium?.copyWith(color: kd.primaryText),
        checkmarkColor: kd.primaryText,
        padding: const EdgeInsets.symmetric(horizontal: 12, vertical: 10),
      ),

      segmentedButtonTheme: SegmentedButtonThemeData(
        style: ButtonStyle(
          minimumSize: const WidgetStatePropertyAll(Size(48, 44)),
          shape: WidgetStatePropertyAll(pill),
          textStyle: WidgetStatePropertyAll(text.labelMedium),
          backgroundColor: WidgetStateProperty.resolveWith((s) => s.contains(WidgetState.selected) ? kd.primaryTint : scheme.surface),
          foregroundColor: WidgetStateProperty.resolveWith((s) => s.contains(WidgetState.selected) ? kd.primaryText : kd.ink2),
          side: WidgetStateProperty.resolveWith((s) => BorderSide(color: s.contains(WidgetState.selected) ? KdPalette.primary : scheme.outlineVariant)),
        ),
      ),

      inputDecorationTheme: InputDecorationTheme(
        filled: true,
        fillColor: scheme.surface,
        contentPadding: const EdgeInsets.symmetric(horizontal: 14, vertical: 14),
        hintStyle: text.bodyLarge?.copyWith(color: kd.ink3),
        border: OutlineInputBorder(borderRadius: BorderRadius.circular(KdSize.radiusControl), borderSide: BorderSide(color: scheme.outline)),
        enabledBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(KdSize.radiusControl), borderSide: BorderSide(color: scheme.outline)),
        focusedBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(KdSize.radiusControl), borderSide: const BorderSide(color: KdPalette.primary, width: 2)),
        errorBorder: OutlineInputBorder(borderRadius: BorderRadius.circular(KdSize.radiusControl), borderSide: const BorderSide(color: KdPalette.danger)),
        errorStyle: text.labelSmall?.copyWith(color: kd.dangerText),
        helperStyle: text.labelSmall,
      ),

      navigationBarTheme: NavigationBarThemeData(
        height: KdSize.navBar,
        backgroundColor: scheme.surface,
        indicatorColor: kd.primaryTint,
        surfaceTintColor: Colors.transparent,
        labelTextStyle: WidgetStateProperty.resolveWith((s) => text.labelSmall?.copyWith(
              fontSize: 11,
              fontWeight: s.contains(WidgetState.selected) ? FontWeight.w600 : FontWeight.w500,
              color: s.contains(WidgetState.selected) ? kd.primaryText : kd.ink2,
            )),
        iconTheme: WidgetStateProperty.resolveWith((s) => IconThemeData(
              size: 22, color: s.contains(WidgetState.selected) ? kd.primaryText : kd.ink2)),
      ),

      bottomSheetTheme: BottomSheetThemeData(
        backgroundColor: scheme.surface,
        surfaceTintColor: Colors.transparent,
        showDragHandle: true,
        dragHandleColor: scheme.outline,
        dragHandleSize: const Size(32, 4),
        modalBarrierColor: scheme.scrim,
        shape: const RoundedRectangleBorder(borderRadius: BorderRadius.vertical(top: Radius.circular(KdSize.radiusSheet))),
      ),

      snackBarTheme: SnackBarThemeData(
        behavior: SnackBarBehavior.floating,
        backgroundColor: kd.inverse,
        contentTextStyle: text.bodyMedium?.copyWith(color: Colors.white),
        actionTextColor: const Color(0xFFC9D0FF),
        closeIconColor: Colors.white,
        shape: RoundedRectangleBorder(borderRadius: BorderRadius.circular(12)),
        insetPadding: const EdgeInsets.fromLTRB(12, 0, 12, 12),
      ),

      switchTheme: SwitchThemeData(
        thumbColor: const WidgetStatePropertyAll(Colors.white),
        trackColor: WidgetStateProperty.resolveWith((s) => s.contains(WidgetState.selected) ? KdPalette.primary : KdPalette.muted),
        trackOutlineColor: const WidgetStatePropertyAll(Colors.transparent),
      ),

      progressIndicatorTheme: ProgressIndicatorThemeData(
        color: KdPalette.primary,
        linearTrackColor: kd.neutralTint,
        circularTrackColor: kd.neutralTint,
      ),

      dividerTheme: DividerThemeData(color: scheme.outlineVariant, thickness: 1, space: 1),
      listTileTheme: ListTileThemeData(
        minVerticalPadding: 12,
        iconColor: KdPalette.primary,
        titleTextStyle: text.bodyLarge?.copyWith(fontWeight: FontWeight.w500),
        subtitleTextStyle: text.labelSmall,
      ),
    );
  }
}

/// Durum çipi için yardımcı. Kullanım: KdStatus.online.chip(context)
enum KdStatus { online, offline, unknown, pending, queued, failed, study, breakTime }

extension KdStatusStyle on KdStatus {
  (Color bg, Color fg, IconData icon, String label) resolve(KdColors kd) => switch (this) {
        KdStatus.online => (kd.successTint, kd.successText, Icons.circle, 'Çevrimiçi'),
        KdStatus.offline => (kd.neutralTint, kd.ink2, Icons.wifi_off, 'Çevrimdışı'),
        KdStatus.unknown => (kd.neutralTint, kd.ink2, Icons.help_outline, 'Bilinmiyor'),
        KdStatus.pending => (kd.warningTint, kd.warningText, Icons.hourglass_top, 'Bekliyor'),
        KdStatus.queued => (kd.warningTint, kd.warningText, Icons.schedule, 'Sırada'),
        KdStatus.failed => (kd.dangerTint, kd.dangerText, Icons.error_outline, 'Başarısız'),
        KdStatus.study => (kd.primaryTint, kd.primaryText, Icons.menu_book, 'Ders modu'),
        KdStatus.breakTime => (kd.secondaryTint, kd.secondaryText, Icons.coffee, 'Mola'),
      };
}

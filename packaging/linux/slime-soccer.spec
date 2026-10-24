Name:           slime-soccer
Version:        %{app_version}
Release:        1
Summary:        Arcade slime football, volleyball and basketball
License:        MIT and Proprietary
BuildArch:      %{app_arch}

# X11 and OpenGL are opened at runtime, so the ELF dependency scanner cannot
# see them and they have to be declared explicitly.
Requires:       libX11.so.6()(64bit)
Requires:       libXcursor.so.1()(64bit)
Requires:       libXi.so.6()(64bit)
Requires:       libXinerama.so.1()(64bit)
Requires:       libXrandr.so.2()(64bit)
Requires:       libGL.so.1()(64bit)

%global debug_package %{nil}

%description
A cross-platform remake of Super Slime Soccer with soccer, volleyball and
basketball rules, local and online play, championships and an English and
Persian interface. The original game artwork is embedded in the executable
and is not MIT licensed; see THIRD_PARTY_NOTICES.md.

%install
install -d %{buildroot}/usr/bin
install -d %{buildroot}/usr/share
install -m 0755 %{payload}/usr/bin/slime-soccer %{buildroot}/usr/bin/slime-soccer
cp -a %{payload}/usr/share/. %{buildroot}/usr/share/

%files
/usr/bin/slime-soccer
/usr/share/applications/slime-soccer.desktop
/usr/share/icons/hicolor/256x256/apps/slime-soccer.png
/usr/share/icons/hicolor/512x512/apps/slime-soccer.png
/usr/share/doc/slime-soccer/

%changelog
* Sun Oct 25 2026 Super Slime Soccer contributors - %{app_version}-1
- Release %{app_version}

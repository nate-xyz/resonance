# SPDX-License-Identifier: GPL-3.0-or-later

project('resonance', 'rust', 
          version: '0.1.3',
    meson_version: '>= 0.59.0',
  default_options: [ 'warning_level=3',
                     'werror=false',
                   ],
)

i18n = import('i18n')
gnome = import('gnome')

dependency('gtk4', version: '>= 4.8.2')
libadwaita = dependency(
  'libadwaita-1',
  version: '>= 1.2.0',
  fallback: ['libadwaita', 'libadwaita_dep'],
  default_options: [
    'examples=false',
    'introspection=disabled',
    'tests=false',
    'vapi=false',
    'default_library=shared',
  ]
)

dependency('gstreamer-1.0', version: '>= 1.16')
dependency('gstreamer-audio-1.0', version: '>= 1.16')
dependency('gstreamer-plugins-base-1.0', version: '>= 1.16')
dependency('gstreamer-plugins-bad-1.0', version: '>= 1.16')
dependency('gstreamer-bad-audio-1.0', version: '>= 1.16')


cargo = find_program('cargo', required: true)

cargo_sources = files(
  'Cargo.toml',
  'Cargo.lock',
)

subdir('data')
subdir('src')
subdir('po')

gnome.post_install(
  glib_compile_schemas: true,
  gtk_update_icon_cache: true,
  update_desktop_database: true,
)

summary({
    'prefix': get_option('prefix'),
    'libdir': get_option('libdir'),
    'datadir': get_option('datadir'),
    'bindir': get_option('bindir'),
  },
  section: 'Directories',
)

import type { Config } from 'tailwindcss'

export default {
  content: ['./index.html', './src/**/*.{vue,ts}'],
  darkMode: 'class',
  theme: {
    extend: {
      fontFamily: { sans: ['Inter', 'sans-serif'] },
      colors: {
        cream: '#FFEEB3',
        mint: '#B4D7CD',
        mist: '#8EA8C3',
        brand: {
          50: '#FFEEB3',
          400: '#F47321',
          500: '#F47321',
          600: '#F47321',
          700: '#d85f12',
        },
        surface: {
          900: '#1c2833',
          800: '#243240',
          700: '#314155',
          600: '#8EA8C3',
        },
      },
    },
  },
  plugins: [],
} satisfies Config

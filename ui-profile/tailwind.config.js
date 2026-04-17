/** @type {import('tailwindcss').Config} */
export default {
  content: [
    "./index.html",
    "./src/**/*.{js,ts,jsx,tsx}",
  ],
  theme: {
    extend: {
      colors: {
        bg: {
          color: 'rgba(10, 10, 10, 0.7)',
        },
        accent: {
          color: '#ffd700',
        },
        text: {
          main: '#ffffff',
          dim: 'rgba(255, 255, 255, 0.4)',
          me: 'rgba(255, 255, 255, 0.15)',
        },
        card: {
          bg: 'rgba(30, 30, 30, 0.85)',
        },
      },
      backdropBlur: {
        xs: '2px',
      },
    },
  },
  plugins: [],
}

/** @type {import('tailwindcss').Config} */
module.exports = {
    darkMode: 'media',
    content: [
    './src/pages/**/*.{js,ts,jsx,tsx,mdx}',
    './src/components/**/*.{js,ts,jsx,tsx,mdx}',
    './src/app/**/*.{js,ts,jsx,tsx,mdx}',
  ],
  theme: {
  	extend: {
  		fontFamily: {
  			// Design system: Open Sans for text, Montserrat for titles and speaker names.
  			sans: ['var(--font-open-sans)', '-apple-system', 'BlinkMacSystemFont', 'sans-serif'],
  			display: ['var(--font-montserrat)', 'var(--font-open-sans)', 'sans-serif']
  		},
  		colors: {
  			background: 'hsl(var(--background))',
  			foreground: 'hsl(var(--foreground))',
  			border: 'hsl(var(--border))',
  			input: 'hsl(var(--input))',
  			ring: 'hsl(var(--ring))',
  			primary: {
  				DEFAULT: 'hsl(var(--primary))',
  				foreground: 'hsl(var(--primary-foreground))'
  			},
  			secondary: {
  				DEFAULT: 'hsl(var(--secondary))',
  				foreground: 'hsl(var(--secondary-foreground))'
  			},
  			rail: {
  				DEFAULT: 'hsl(var(--rail))',
  				foreground: 'hsl(var(--rail-foreground))',
  				muted: 'hsl(var(--rail-muted))',
  				border: 'hsl(var(--rail-border))',
  				active: 'hsl(var(--rail-active))',
  				accent: 'hsl(var(--rail-accent))',
  				live: 'hsl(var(--rail-live))'
  			},
  			success: 'hsl(var(--success))',
  			live: {
  				DEFAULT: 'hsl(var(--live))',
  				foreground: 'hsl(var(--live-foreground))'
  			},
  			speaker: {
  				me: 'hsl(var(--speaker-me))',
  				'1': 'hsl(var(--speaker-1))',
  				'2': 'hsl(var(--speaker-2))',
  				'3': 'hsl(var(--speaker-3))',
  				'4': 'hsl(var(--speaker-4))',
  				'5': 'hsl(var(--speaker-5))',
  				'6': 'hsl(var(--speaker-6))',
  				'7': 'hsl(var(--speaker-7))',
  				other: 'hsl(var(--speaker-other))'
  			},
  			card: {
  				DEFAULT: 'hsl(var(--card))',
  				foreground: 'hsl(var(--card-foreground))'
  			},
  			popover: {
  				DEFAULT: 'hsl(var(--popover))',
  				foreground: 'hsl(var(--popover-foreground))'
  			},
  			muted: {
  				DEFAULT: 'hsl(var(--muted))',
  				foreground: 'hsl(var(--muted-foreground))'
  			},
  			accent: {
  				DEFAULT: 'hsl(var(--accent))',
  				foreground: 'hsl(var(--accent-foreground))'
  			},
  			destructive: {
  				DEFAULT: 'hsl(var(--destructive))',
  				foreground: 'hsl(var(--destructive-foreground))'
  			},
  		},
  		borderRadius: {
  			lg: 'var(--radius)',
  			md: 'calc(var(--radius) - 2px)',
  			sm: 'calc(var(--radius) - 4px)'
  		},
  		keyframes: {
  			'accordion-down': {
  				from: {
  					height: '0'
  				},
  				to: {
  					height: 'var(--radix-accordion-content-height)'
  				}
  			},
  			'accordion-up': {
  				from: {
  					height: 'var(--radix-accordion-content-height)'
  				},
  				to: {
  					height: '0'
  				}
  			}
  		},
  		animation: {
  			'accordion-down': 'accordion-down 0.2s ease-out',
  			'accordion-up': 'accordion-up 0.2s ease-out'
  		}
  	}
  },
  plugins: [
    require("tailwindcss-animate"),
    require("@tailwindcss/typography"),
    require("@tailwindcss/container-queries"),
  ],
}
// @ts-check
// Note: type annotations allow type checking and IDEs autocompletion

const lightCodeTheme = require("prism-react-renderer/themes/github");
const darkCodeTheme = require("prism-react-renderer/themes/dracula");

/** @type {import('@docusaurus/types').Config} */
const config = {
	title: "Pisanix",
	tagline: "A Database Mesh Solution sponsored by SphereEx",
	url: "https://www.pisanix.io",
	baseUrl: "/",
	onBrokenLinks: "throw",
	onBrokenMarkdownLinks: "warn",
	favicon: "img/favicon.ico",
	organizationName: "database-mesh",
	projectName: "pisanix",
	i18n: {
		defaultLocale: "zh",
		locales: ["en", "zh"],
	},

	presets: [
		[
			"classic",
			/** @type {import('@docusaurus/preset-classic').Options} */
			({
				docs: {
					sidebarPath: require.resolve("./sidebars.js"),
          showLastUpdateAuthor: true,
          showLastUpdateTime: true,
          includeCurrentVersion: true,
          lastVersion: 'current',
          versions: {
            current: {
              label: 'latest',
            },
          },
				},
				blog: {
					showReadingTime: true,
				},
				theme: {
					customCss: require.resolve("./src/css/custom.css"),
				},
			}),
		],
	],

	themeConfig:
		/** @type {import('@docusaurus/preset-classic').ThemeConfig} */
		({
			navbar: {
				// title: 'Pisanix',
				logo: {
					alt: "Pisanix Logo",
					src: "img/logo.svg",
				},
				items: [
					{
					  type: 'docsVersionDropdown',
						position: "right",
					},
					{
						type: "localeDropdown",
						position: "right",
					},
					{
						type: "doc",
						docId: "intro",
						position: "left",
						label: "文档",
					},
					{ to: "/blog", label: "博客", position: "left" },
					{ to: "/doc", type: "doc", docId: "UseCases/kubernetes", label: "使用场景", position: "left" },
					{
						href: "https://github.com/database-mesh/pisanix",
						label: "GitHub",
						position: "right",
					},
				],
			},
			footer: {
				style: "dark",
				links: [
				{
						title: "Docs",
						items: [
							{
								label: "文档",
								to: "/docs/intro",
							},
							{
								label: "快速开始",
								to: "/docs/quickstart",
							},
						],
					},
					{
						title: "社区",
						items: [
							{
								label: "Twitter",
								href: "https://twitter.com/maxwell9215",
							},
							{
								label: "Slack",
								href: "https://databasemesh.slack.com/",
							},
						],
					},
					{
						title: "更多",
						items: [
							{
								label: "博客",
								to: "/blog",
							},
							{
								label: "GitHub",
								href: "https://github.com/database-mesh/pisanix",
							},
						],
					},
				],
				copyright: `Copyright © ${new Date().getFullYear()} SphereEx Authors. Built with Docusaurus.`,
			},
			prism: {
				theme: lightCodeTheme,
				darkTheme: darkCodeTheme,
			},
		}),
};

module.exports = config;

<script>
	import Github from '~/icons/Github.svelte'
	import { i18n } from '$lib/i18n'
	import { base } from '$app/paths'
	import PrivacyPolicy from './PrivacyPolicy.svelte'
	import KofiDialog from './KofiDialog.svelte'
	import Discord from '~/icons/Discord.svelte'

	const t = $i18n.t
</script>

<footer class="footer footer-center mt-36 p-10 bg-base-200 text-base-content rounded">
	<PrivacyPolicy />
	<KofiDialog />

	<nav class="flex flex-row flex-wrap gap-4 justify-center">
		<a class="link link-hover" href={base}>{t('home')}</a>
		<button
			on:click={() => {
				// @ts-ignore
				window?.['kofi-dialog'].showModal()
			}}
			class="link link-hover">{t('support-vibe')}</button>
		<button
			on:click={() => {
				// @ts-ignore
				window['privacy-policy-modal']?.showModal()
			}}
			class="link link-hover">{t('privacy-policy')}</button>
		<a class="link link-hover" href={`${base}/features`}>{t('features')}</a>

		<a class="link link-hover" href="/vibe/docs">{t('documentation')}</a>
	</nav>
	<nav>
		<div class="grid grid-flow-col gap-4">
			<a href="https://github.com/thewh1teagle/vibe" target="_blank">
				<Github width="24" height="24" />
			</a>
			<div class="divider lg:divider-horizontal !p-0 !m-0" />
			<a href={`https://discord.gg/EcxWSstQN8`} target="_blank">
				<Discord />
			</a>
		</div>
	</nav>
	<aside>
		<p>Vibe - {t('title')}</p>
	</aside>
</footer>

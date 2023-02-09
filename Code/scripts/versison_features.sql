--
-- PostgreSQL database dump
--

-- Dumped from database version 14.2
-- Dumped by pg_dump version 14.2

SET statement_timeout = 0;
SET lock_timeout = 0;
SET idle_in_transaction_session_timeout = 0;
SET client_encoding = 'UTF8';
SET standard_conforming_strings = on;
SELECT pg_catalog.set_config('search_path', '', false);
SET check_function_bodies = false;
SET xmloption = content;
SET client_min_messages = warning;
SET row_security = off;

SET default_tablespace = '';

SET default_table_access_method = heap;

--
-- Name: version_feature; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.version_feature (
    id integer,
    conds character varying(255),
    feature character varying(40) DEFAULT 'no_feature_used'::character varying
);


ALTER TABLE public.version_feature OWNER TO postgres;

--
-- Data for Name: version_feature; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.version_feature (id, conds, feature) FROM stdin;
137718	all(test,windows)	std_misc
348229	\N	no_feature_used
595415	doc_cfg	doc_cfg
11290	\N	no_feature_used
54370	\N	no_feature_used
532794	\N	no_feature_used
564790	\N	no_feature_used
468088	\N	no_feature_used
11665	\N	no_feature_used
524851	\N	no_feature_used
541877	\N	no_feature_used
111488	\N	no_feature_used
557605	\N	no_feature_used
581500	\N	no_feature_used
150941	\N	no_feature_used
436226	\N	no_feature_used
504345	\N	no_feature_used
569468	docsrs	doc_cfg
602010	docsrs	doc_cfg
180133	\N	no_feature_used
557298	\N	no_feature_used
142579	\N	no_feature_used
388094	\N	no_feature_used
388093	\N	no_feature_used
7903	\N	no_feature_used
598989	__time_03_docs	doc_cfg
598989	__time_03_docs	doc_auto_cfg
598989	__time_03_docs	doc_notable_trait
8814	test	test
592704	\N	no_feature_used
8813	test	test
8817	test	test
8815	test	test
32652	test	test
8816	test	test
202034	\N	no_feature_used
592705	\N	no_feature_used
375517	\N	no_feature_used
601970	feature = bench	test
510162	docsrs	doc_cfg
586477	all(test,feature = benchmarks)	test
184136	\N	no_feature_used
350914	\N	no_feature_used
327835	\N	no_feature_used
348795	\N	no_feature_used
562254	\N	no_feature_used
570828	\N	no_feature_used
581503	\N	no_feature_used
27401	feature = with-bench	test
561939	\N	no_feature_used
526415	\N	no_feature_used
501531	docsrs	doc_cfg
584347	docsrs	doc_cfg
474281	all(nightly,doc)	doc_cfg
463966	\N	no_feature_used
480432	docsrs	doc_cfg
570767	\N	no_feature_used
598155	\N	no_feature_used
580569	all(test,feature = nightly)	test
580569	docsrs	doc_cfg
593875		bench_black_box
598158	\N	no_feature_used
203510	lazy_static_spin_impl	const_fn
183578	\N	no_feature_used
173412	\N	no_feature_used
171925	\N	no_feature_used
383566	feature = clippy	plugin
383566	feature = bench	test
412367	\N	no_feature_used
596909	\N	no_feature_used
27140	\N	no_feature_used
606237	feature = unstable	never_type
256324	\N	no_feature_used
406246	\N	no_feature_used
50893	rustbuild	staged_api
50893	rustbuild	rustc_private
50893	rust_build	staged_api
291256	\N	no_feature_used
579113	feature = pattern	pattern
170554	\N	no_feature_used
228521	\N	no_feature_used
542797	rustbuild	staged_api
542797	rustbuild	rustc_private
533482	\N	no_feature_used
500982	\N	no_feature_used
588693	\N	no_feature_used
588669	\N	no_feature_used
602449	\N	no_feature_used
464395	\N	no_feature_used
533118	\N	no_feature_used
587146	docsrs	doc_cfg
326545	\N	no_feature_used
189280	feature = nightly	test
501552	docsrs	doc_cfg
557673	\N	no_feature_used
477982	\N	no_feature_used
413602	\N	no_feature_used
45907	\N	no_feature_used
516398	feature = docsrs	doc_cfg
516398	feature = docsrs	extended_key_value_attributes
403812	\N	no_feature_used
25677	\N	no_feature_used
603667	feature = rustc-dep-of-std	link_cfg
603667	feature = rustc-dep-of-std	no_core
603667	libc_thread_local	thread_local
603667	libc_const_extern_fn_unstable	const_extern_fn
56891	\N	no_feature_used
381875	\N	no_feature_used
598157	\N	no_feature_used
503729	\N	no_feature_used
18007	\N	no_feature_used
584224	docsrs	doc_cfg
401118	rustbuild	staged_api
401118	rustbuild	rustc_private
593357	\N	no_feature_used
292957	\N	no_feature_used
153940	feature = rustc-dep-of-std	no_core
499768	feature = simd_support	stdsimd
499768	doc_cfg	doc_cfg
350218	\N	no_feature_used
592707	\N	no_feature_used
208351	\N	no_feature_used
25788	\N	no_feature_used
583526	\N	no_feature_used
447793	\N	no_feature_used
496510	\N	no_feature_used
539401	\N	no_feature_used
603806	\N	no_feature_used
440895	\N	no_feature_used
598156	\N	no_feature_used
598160	\N	no_feature_used
606532	feature = nightly	thread_id_value
85712	\N	no_feature_used
240128	\N	no_feature_used
45630	\N	no_feature_used
585953	\N	no_feature_used
510957	\N	no_feature_used
601137	docsrs	doc_auto_cfg
536682	\N	no_feature_used
470755	\N	no_feature_used
588992	\N	no_feature_used
573176	all(feature = nightly,test)	test
140563	\N	no_feature_used
445178	target_os = wasi	thread_local
256812	\N	no_feature_used
43760	\N	no_feature_used
575775	docsrs	doc_cfg
575775	feature = specialization	specialization
575775	feature = may_dangle	dropck_eyepatch
577969	\N	no_feature_used
297871	\N	no_feature_used
241390	\N	no_feature_used
604586	\N	no_feature_used
202895	\N	no_feature_used
479610	feature = nightly	wasi_ext
426562	feature = bench	test
496087	\N	no_feature_used
576728	\N	no_feature_used
26776	\N	no_feature_used
424645	\N	no_feature_used
595084	\N	no_feature_used
407038	\N	no_feature_used
352710	\N	no_feature_used
576455	\N	no_feature_used
542524	feature = bench	test
542524	feature = bench	unicode_internals
523001	\N	no_feature_used
213235	\N	no_feature_used
574869	\N	no_feature_used
333591	\N	no_feature_used
584832	coverage	no_coverage
238420	\N	no_feature_used
576947	\N	no_feature_used
587013	docsrs	doc_cfg
587013	bench	test
109920	\N	no_feature_used
543679	\N	no_feature_used
127959	\N	no_feature_used
599360	\N	no_feature_used
316700	nightly	try_trait
534624	\N	no_feature_used
445776	\N	no_feature_used
181407	\N	no_feature_used
13939	\N	no_feature_used
72203	feature = clippy	plugin
72203	all(feature = bench,test)	test
72203	feature = simd	platform_intrinsics
72203	feature = simd	repr_simd
72203	feature = simd_opt	cfg_target_feature
72203	feature = simd_asm	asm
590916	\N	no_feature_used
508757	\N	no_feature_used
171631	\N	no_feature_used
291830	\N	no_feature_used
52846	\N	no_feature_used
579110	\N	no_feature_used
311948	\N	no_feature_used
589517	all(test,feature = nightly)	test
573823	\N	no_feature_used
591437	rustbuild	staged_api
591437	rustbuild	rustc_private
321930	\N	no_feature_used
539154	\N	no_feature_used
595217	docsrs	doc_cfg
541769	\N	no_feature_used
580367	all(feature = std,target_env = sgx,target_vendor = fortanix)	sgx_platform
373218	\N	no_feature_used
236348	\N	no_feature_used
586771	\N	no_feature_used
523018	\N	no_feature_used
602969	\N	no_feature_used
186581		test
527090	\N	no_feature_used
494822	test	test
418406	\N	no_feature_used
523010	all(test,feature = bench)	test
504587	docsrs	doc_cfg
501113	\N	no_feature_used
290855	\N	no_feature_used
201392	\N	no_feature_used
602105	all(test,feature = bench_private)	test
107280	\N	no_feature_used
602493	\N	no_feature_used
543249	\N	no_feature_used
599261	docsrs	doc_cfg
595703	\N	no_feature_used
544073	\N	no_feature_used
606416	\N	no_feature_used
13777	\N	no_feature_used
478313	\N	no_feature_used
216006	\N	no_feature_used
544074	\N	no_feature_used
344284	\N	no_feature_used
606575	docsrs	doc_cfg
108925	\N	no_feature_used
589255	\N	no_feature_used
387021	\N	no_feature_used
81896	all(feature = nightly,test)	test
81896	feature = nightly	drop_types_in_const
81896	all(feature = nightly,test)	core_intrinsics
81896	feature = nightly	const_fn
81896	feature = nightly	const_unsafe_cell_new
207011	\N	no_feature_used
534625	\N	no_feature_used
268003	\N	no_feature_used
578460	\N	no_feature_used
589600	\N	no_feature_used
76612	test	test
589159	docsrs	doc_cfg
119232	\N	no_feature_used
585706	docsrs	doc_cfg
375502	\N	no_feature_used
604541	\N	no_feature_used
504580	\N	no_feature_used
355237	\N	no_feature_used
78551	\N	no_feature_used
472975	\N	no_feature_used
595100	\N	no_feature_used
413374	\N	no_feature_used
584829	\N	no_feature_used
584831	\N	no_feature_used
519471	\N	no_feature_used
527368	\N	no_feature_used
437270	all(feature = mac_os_10_7_support,feature = mac_os_10_8_features)	linkage
129206	feature = unstable	test
551087	\N	no_feature_used
346883	docsrs	doc_cfg
450611	\N	no_feature_used
460189	\N	no_feature_used
367377	\N	no_feature_used
568662	\N	no_feature_used
483075	libloading_docs	doc_cfg
36501	\N	no_feature_used
488214	\N	no_feature_used
426129	\N	no_feature_used
226794	\N	no_feature_used
495257	\N	no_feature_used
604979	\N	no_feature_used
467772	\N	no_feature_used
588937	feature = unstable	trait_alias
588937	doc_cfg	doc_cfg
588937	doc_cfg	doc_auto_cfg
524843	feature = benchmark	test
524843	feature = no-stdlib-ffi-binding,not(feature = std)	lang_items
332962	\N	no_feature_used
548779	\N	no_feature_used
257403	\N	no_feature_used
557495	\N	no_feature_used
600466	\N	no_feature_used
593631	\N	no_feature_used
263309	\N	no_feature_used
403357	\N	no_feature_used
555085	feature = alloc_trait	allocator_api
304302	\N	no_feature_used
572607	\N	no_feature_used
585035	docsrs	doc_cfg
551841	\N	no_feature_used
244297	\N	no_feature_used
555468	\N	no_feature_used
166593	feature = nightly	alloc
166593	feature = nightly	read_initializer
166593	feature = nightly	specialization
166593	all(test,feature = nightly)	io
166593	all(test,feature = nightly)	test
421570	\N	no_feature_used
85551	\N	no_feature_used
545732	\N	no_feature_used
335685	\N	no_feature_used
335512	\N	no_feature_used
547509	feature = doc-cfg	doc_cfg
584620	\N	no_feature_used
60254	\N	no_feature_used
606572	\N	no_feature_used
583849	\N	no_feature_used
100784	\N	no_feature_used
27486	\N	no_feature_used
596926	\N	no_feature_used
356040	\N	no_feature_used
271883	feature = nightly	plugin
262200	\N	no_feature_used
586036	all(feature = use-intrinsics,any(target_arch = x86,target_arch = x86_64))	stdsimd
586036	all(feature = use-intrinsics,any(target_arch = x86,target_arch = x86_64))	f16c_target_feature
586036	docsrs	doc_cfg
594854	\N	no_feature_used
562063	\N	no_feature_used
135974	\N	no_feature_used
70631	\N	no_feature_used
65780	\N	no_feature_used
562064	\N	no_feature_used
601064	\N	no_feature_used
456327	\N	no_feature_used
367299	\N	no_feature_used
451592	\N	no_feature_used
459796	\N	no_feature_used
572455	\N	no_feature_used
534747	\N	no_feature_used
602425	\N	no_feature_used
597674	\N	no_feature_used
488544	\N	no_feature_used
541481	\N	no_feature_used
542873	\N	no_feature_used
568600	\N	no_feature_used
539395	\N	no_feature_used
572197	\N	no_feature_used
447943	\N	no_feature_used
606288	\N	no_feature_used
501532	docsrs	doc_cfg
268802	\N	no_feature_used
138234	feature = alloc	alloc
323042	\N	no_feature_used
602660	\N	no_feature_used
417029	\N	no_feature_used
558669	\N	no_feature_used
122631	\N	no_feature_used
327060	\N	no_feature_used
407186	\N	no_feature_used
275637	\N	no_feature_used
569851	doc_cfg	doc_cfg
571257	\N	no_feature_used
400869	\N	no_feature_used
222315	\N	no_feature_used
433599	\N	no_feature_used
569169	\N	no_feature_used
560545	\N	no_feature_used
170628	\N	no_feature_used
585609	\N	no_feature_used
595704	\N	no_feature_used
170277	\N	no_feature_used
606235	\N	no_feature_used
595694	\N	no_feature_used
493797	\N	no_feature_used
590734	docsrs	doc_cfg
237255	\N	no_feature_used
504182	\N	no_feature_used
527627	feature = simd-accel	stdsimd
527627	feature = simd-accel	core_intrinsics
520104	\N	no_feature_used
505762	\N	no_feature_used
436527	\N	no_feature_used
594853	\N	no_feature_used
520884	\N	no_feature_used
602456	docsrs	doc_cfg
602447	docsrs	doc_cfg
76110	\N	no_feature_used
373013	\N	no_feature_used
535570	\N	no_feature_used
498146	all(test,feature = nightly)	test
363906	\N	no_feature_used
594851	\N	no_feature_used
541705	all(target_env = sgx,target_vendor = fortanix)	sgx_platform
541705	all(feature = nightly,target_family = wasm,target_feature = atomics)	stdsimd
571728	\N	no_feature_used
551129	docsrs	doc_cfg
551129	read_buf	read_buf
236841	\N	no_feature_used
581346	\N	no_feature_used
595420	\N	no_feature_used
595419	doc_cfg	doc_cfg
312164	\N	no_feature_used
258137	\N	no_feature_used
184217	\N	no_feature_used
209537	\N	no_feature_used
484373	\N	no_feature_used
469452	\N	no_feature_used
402571	\N	no_feature_used
203275	all(feature = nightly,test)	test
451545	all(feature = benchmarks,test)	test
541638	\N	no_feature_used
118376	\N	no_feature_used
603855	\N	no_feature_used
326604	\N	no_feature_used
300224	\N	no_feature_used
587296	all(feature = nightly,test)	test
559781	\N	no_feature_used
511188	\N	no_feature_used
500634	docsrs	doc_cfg
130507	\N	no_feature_used
580573	docsrs	doc_cfg
580573	all(feature = neon,target_arch = aarch64,target_feature = neon)	stdsimd
580573	all(feature = neon,target_arch = aarch64,target_feature = neon)	aarch64_target_feature
593569	\N	no_feature_used
599731	docsrs	doc_cfg
599702	docsrs	doc_cfg
501549	docsrs	doc_cfg
502244	feature = simd	platform_intrinsics
502244	feature = simd	repr_simd
501482	docsrs	doc_cfg
501482	all(aes_armv8,target_arch = aarch64)	stdsimd
501482	all(aes_armv8,target_arch = aarch64)	aarch64_target_feature
550084	docsrs	doc_cfg
594086	\N	no_feature_used
594112	\N	no_feature_used
40586	\N	no_feature_used
330894	\N	no_feature_used
337205	\N	no_feature_used
456547	docsrs	doc_cfg
436039	\N	no_feature_used
564692	\N	no_feature_used
590665	\N	no_feature_used
403302	docsrs	doc_cfg
454785	\N	no_feature_used
521733	\N	no_feature_used
229717	feature = bench	test
566169	docsrs	doc_cfg
35608	\N	no_feature_used
523006	\N	no_feature_used
302497	docsrs	doc_cfg
388963	\N	no_feature_used
481888	\N	no_feature_used
409301	\N	no_feature_used
328115	\N	no_feature_used
336333	\N	no_feature_used
523660	docsrs	doc_cfg
603138	\N	no_feature_used
380583	\N	no_feature_used
597060	\N	no_feature_used
376253	\N	no_feature_used
537525	feature = nightly-testing	plugin
509180	\N	no_feature_used
476207	\N	no_feature_used
584836	docsrs	doc_auto_cfg
203072	\N	no_feature_used
510956	\N	no_feature_used
585618	\N	no_feature_used
286087	\N	no_feature_used
596906	\N	no_feature_used
501130	feature = nightly	test
501130	feature = nightly	doc_cfg
501130	feature = simd_backend	stdsimd
462079	all(not(feature = std),feature = alloc)	alloc
77853	\N	no_feature_used
175816	\N	no_feature_used
216593	\N	no_feature_used
77854	\N	no_feature_used
507523	\N	no_feature_used
257634	\N	no_feature_used
238527	\N	no_feature_used
91173	\N	no_feature_used
440209	\N	no_feature_used
415662	\N	no_feature_used
606538	\N	no_feature_used
568646	docsrs	doc_cfg
600031	\N	no_feature_used
207836	\N	no_feature_used
254095	\N	no_feature_used
354012	\N	no_feature_used
586746	feature = nightly	negative_impls
586746	feature = nightly	auto_traits
320147	\N	no_feature_used
542861	\N	no_feature_used
203094	\N	no_feature_used
474307	\N	no_feature_used
209528	\N	no_feature_used
386659	\N	no_feature_used
571051	\N	no_feature_used
182405	\N	no_feature_used
476206	\N	no_feature_used
439362	\N	no_feature_used
484995	\N	no_feature_used
454554	\N	no_feature_used
540252	\N	no_feature_used
276173	\N	no_feature_used
595716	\N	no_feature_used
250827	\N	no_feature_used
565640	docsrs	doc_cfg
428228	\N	no_feature_used
406650	\N	no_feature_used
597879	\N	no_feature_used
543711	\N	no_feature_used
496267	\N	no_feature_used
523016	\N	no_feature_used
46682	\N	no_feature_used
518838		test
518838	docsrs	doc_cfg
378072	\N	no_feature_used
478612	\N	no_feature_used
554872	docsrs	doc_cfg
261873	\N	no_feature_used
603924	\N	no_feature_used
562127	docsrs	doc_auto_cfg
548493	\N	no_feature_used
583905	\N	no_feature_used
437978	\N	no_feature_used
181750	all(test,rust_nightly)	linkage
181750	rust_nightly	core_intrinsics
181750	feature = nightly	never_type
181526	\N	no_feature_used
209538	\N	no_feature_used
583826	\N	no_feature_used
209541	\N	no_feature_used
583839	\N	no_feature_used
67070	\N	no_feature_used
535567	\N	no_feature_used
516621	\N	no_feature_used
465743	\N	no_feature_used
72177	\N	no_feature_used
348178	\N	no_feature_used
501529	\N	no_feature_used
417914	\N	no_feature_used
525984	\N	no_feature_used
595732	\N	no_feature_used
576806	\N	no_feature_used
555906	\N	no_feature_used
464597	docsrs	doc_cfg
563902	\N	no_feature_used
557056	\N	no_feature_used
540937	has_specialisation	specialization
363845	\N	no_feature_used
171505	\N	no_feature_used
593343	docsrs	doc_cfg
499431	\N	no_feature_used
407181	feature = no-stdlib-ffi-binding,not(feature = std)	lang_items
407181	feature = no-stdlib-ffi-binding,not(feature = std)	panic_handler
572135	\N	no_feature_used
599181	\N	no_feature_used
328168	\N	no_feature_used
601410	\N	no_feature_used
591930	\N	no_feature_used
140185	\N	no_feature_used
554017	\N	no_feature_used
592719	\N	no_feature_used
540442	feature = diagnostics	proc_macro_diagnostic
540443	\N	no_feature_used
540444	\N	no_feature_used
421380	\N	no_feature_used
202778	\N	no_feature_used
537528	feature = unstable	proc_macro
537528	feature = nightly-testing	plugin
212549	\N	no_feature_used
61249	\N	no_feature_used
435878	\N	no_feature_used
537713	\N	no_feature_used
468155	docsrs	doc_cfg
368766	\N	no_feature_used
400585	\N	no_feature_used
413991	\N	no_feature_used
520765	\N	no_feature_used
459612	\N	no_feature_used
481549	\N	no_feature_used
547508	feature = doc-cfg	doc_cfg
167498	\N	no_feature_used
497023	\N	no_feature_used
475407	\N	no_feature_used
339090	\N	no_feature_used
342488	feature = unstable	allocator_api
342488	feature = unstable	try_trait
342488	feature = unstable	generator_trait
342488	feature = unstable	never_type
342488	feature = unstable	try_reserve
342488	all(feature = std,feature = unstable)	ip
342488	all(feature = alloc,not(feature = std))	core_intrinsics
593103	\N	no_feature_used
545588	docsrs	doc_cfg
593105	\N	no_feature_used
604368	\N	no_feature_used
595254	\N	no_feature_used
543870	docsrs	doc_cfg
524666	docsrs	doc_cfg
595416	any(proc_macro_span,super_unstable)	proc_macro_span
595416	super_unstable	proc_macro_def_site
595416	doc_cfg	doc_cfg
487851	\N	no_feature_used
589623	\N	no_feature_used
257829	\N	no_feature_used
430495	\N	no_feature_used
364074	\N	no_feature_used
321510	\N	no_feature_used
148976	\N	no_feature_used
201836	\N	no_feature_used
364077	\N	no_feature_used
329361	\N	no_feature_used
462226	\N	no_feature_used
549724	feature = nightly	auto_traits
549724	feature = nightly	negative_impls
549724	docsrs	doc_cfg
549724	docsrs	doc_auto_cfg
603453	\N	no_feature_used
374039	\N	no_feature_used
61450	\N	no_feature_used
186876	\N	no_feature_used
458503	\N	no_feature_used
533753	\N	no_feature_used
506490	\N	no_feature_used
606292	\N	no_feature_used
585919	docsrs	doc_cfg
589595	\N	no_feature_used
457514	\N	no_feature_used
595255	\N	no_feature_used
252987	\N	no_feature_used
390019	doc_cfg	doc_cfg
136895	feature = exact-size-is-empty	exact_size_is_empty
136895	feature = trusted-len	trusted_len
430815	\N	no_feature_used
578962	\N	no_feature_used
136896	\N	no_feature_used
429330	feature = bench	test
508244	\N	no_feature_used
76516	\N	no_feature_used
213336	\N	no_feature_used
594588	\N	no_feature_used
499449	\N	no_feature_used
524665		test
524665	docsrs	doc_cfg
71706	\N	no_feature_used
237062	feature = small-error	extern_types
237062	feature = small-error	allocator_api
205786	\N	no_feature_used
595712	\N	no_feature_used
595713	\N	no_feature_used
462561	feature = unstable_const	const_ptr_offset_from
462561	feature = unstable_const	const_refs_to_cell
264693	\N	no_feature_used
531141	\N	no_feature_used
565084	docsrs	doc_cfg
581980	\N	no_feature_used
252420	\N	no_feature_used
606528	\N	no_feature_used
584464	\N	no_feature_used
237063	\N	no_feature_used
354716	\N	no_feature_used
251302	\N	no_feature_used
589598	\N	no_feature_used
589597	\N	no_feature_used
589599	\N	no_feature_used
520213	\N	no_feature_used
593148	docsrs	doc_cfg
588936	\N	no_feature_used
396663	\N	no_feature_used
579943	feature = real_blackbox	test
426880	\N	no_feature_used
198100	\N	no_feature_used
194476	\N	no_feature_used
393795	\N	no_feature_used
560643	\N	no_feature_used
322769	\N	no_feature_used
494780	\N	no_feature_used
203914	\N	no_feature_used
606109	\N	no_feature_used
600127	\N	no_feature_used
570612	\N	no_feature_used
500944	\N	no_feature_used
575944	docsrs	doc_cfg
561150	docsrs	doc_cfg
209539	\N	no_feature_used
209536	\N	no_feature_used
209540	\N	no_feature_used
209533	\N	no_feature_used
417150	\N	no_feature_used
136894	feature = unstable	unicode_version
209527	\N	no_feature_used
136897	\N	no_feature_used
136913	\N	no_feature_used
136944	\N	no_feature_used
209529	\N	no_feature_used
209542	\N	no_feature_used
568645	\N	no_feature_used
514121	docsrs	doc_cfg
313195	\N	no_feature_used
181749	\N	no_feature_used
181748	\N	no_feature_used
82234	\N	no_feature_used
108599	\N	no_feature_used
245908	\N	no_feature_used
602446	\N	no_feature_used
602452	\N	no_feature_used
602448	\N	no_feature_used
602454	feature = write-all-vectored	io_slice_advance
602454	docsrs	doc_cfg
602455	docsrs	doc_cfg
579015	\N	no_feature_used
374040	\N	no_feature_used
590811	\N	no_feature_used
590814	\N	no_feature_used
579014	\N	no_feature_used
590823	\N	no_feature_used
261264	\N	no_feature_used
170801	\N	no_feature_used
495984	\N	no_feature_used
555081	feature = simd	portable_simd
569839	\N	no_feature_used
524607	\N	no_feature_used
258109	\N	no_feature_used
590827	\N	no_feature_used
539002	\N	no_feature_used
344186	\N	no_feature_used
450792	\N	no_feature_used
593590	docsrs	doc_cfg
478214	\N	no_feature_used
274944	\N	no_feature_used
570830	\N	no_feature_used
569010	\N	no_feature_used
210742	\N	no_feature_used
436645	\N	no_feature_used
599180	\N	no_feature_used
553936	\N	no_feature_used
482365	\N	no_feature_used
496623	\N	no_feature_used
199886	\N	no_feature_used
606453	\N	no_feature_used
524628	\N	no_feature_used
606206	\N	no_feature_used
563574	\N	no_feature_used
590345	\N	no_feature_used
581985	\N	no_feature_used
550138	docsrs	doc_cfg
606110	\N	no_feature_used
543878	\N	no_feature_used
581880	\N	no_feature_used
587744	\N	no_feature_used
587755	\N	no_feature_used
587753	\N	no_feature_used
587749	\N	no_feature_used
590825	\N	no_feature_used
593129	docsrs	doc_cfg
599968	all(feature = unstable)	core_intrinsics
554399	docsrs	doc_cfg
590826	\N	no_feature_used
590815	\N	no_feature_used
590816	\N	no_feature_used
510521	\N	no_feature_used
595361	\N	no_feature_used
587554	\N	no_feature_used
501600	docsrs	doc_cfg
481548	\N	no_feature_used
481550	\N	no_feature_used
603577	\N	no_feature_used
587748	\N	no_feature_used
233802	\N	no_feature_used
388852	\N	no_feature_used
333468	\N	no_feature_used
450369	\N	no_feature_used
455886	\N	no_feature_used
590813	\N	no_feature_used
590824	target_feature = atomics	stdsimd
233133	\N	no_feature_used
587746	\N	no_feature_used
242788	not(feature = std)	alloc
179343	\N	no_feature_used
168208	\N	no_feature_used
195986	\N	no_feature_used
589596	\N	no_feature_used
498318	\N	no_feature_used
213329	\N	no_feature_used
606291	\N	no_feature_used
606289	\N	no_feature_used
210825	\N	no_feature_used
587601	docsrs	doc_cfg
402658	docsrs	doc_cfg
427993	\N	no_feature_used
316407	\N	no_feature_used
575670	\N	no_feature_used
390020	\N	no_feature_used
388087	\N	no_feature_used
316406	\N	no_feature_used
390021	\N	no_feature_used
603196	docsrs	doc_cfg
115234	\N	no_feature_used
551494	\N	no_feature_used
585744	feature = nightly	test
585744	feature = nightly	core_intrinsics
585744	feature = nightly	dropck_eyepatch
585744	feature = nightly	min_specialization
585744	feature = nightly	extend_one
585744	feature = nightly	allocator_api
585744	feature = nightly	slice_ptr_get
585744	feature = nightly	nonnull_slice_from_raw_parts
585744	feature = nightly	maybe_uninit_array_assume_init
585744	feature = nightly	build_hasher_simple_hash_one
584425	docsrs	doc_auto_cfg
602536	docsrs	doc_cfg
254353	\N	no_feature_used
595354	\N	no_feature_used
531140	\N	no_feature_used
510359	\N	no_feature_used
327134	\N	no_feature_used
496640	all(feature = nightly,target_arch = aarch64)	stdsimd
496640	all(feature = nightly,target_arch = aarch64)	aarch64_target_feature
599970	not(feature = no-asm)	asm
599970		abi_unadjusted
599970	not(feature = no-asm)	global_asm
599970		cfg_target_has_atomic
599970		compiler_builtins
599970		core_ffi_c
599970		core_intrinsics
599970		inline_const
599970		lang_items
599970		linkage
599970		naked_functions
599970		repr_simd
604443	feature = allocator_api	allocator_api
604443	feature = allocator_api	nonnull_slice_from_raw_parts
508452	\N	no_feature_used
552470	\N	no_feature_used
494781	\N	no_feature_used
496410	\N	no_feature_used
592724	\N	no_feature_used
451396	\N	no_feature_used
597427	\N	no_feature_used
469909	\N	no_feature_used
500095	\N	no_feature_used
513122	\N	no_feature_used
367578	\N	no_feature_used
551493	\N	no_feature_used
513142	\N	no_feature_used
595701	\N	no_feature_used
451022	\N	no_feature_used
476440	\N	no_feature_used
589673	\N	no_feature_used
600165	docsrs	doc_cfg
600165	docsrs	doc_auto_cfg
600164	docsrs	doc_cfg
316408	\N	no_feature_used
602929	\N	no_feature_used
556437	docsrs	doc_cfg
529131	\N	no_feature_used
172422	\N	no_feature_used
595718	\N	no_feature_used
514107	\N	no_feature_used
566250	docsrs	doc_cfg
587665	\N	no_feature_used
515071	\N	no_feature_used
347185	\N	no_feature_used
606556	docsrs	doc_cfg
132146	\N	no_feature_used
209534	\N	no_feature_used
542831	\N	no_feature_used
521253	\N	no_feature_used
542832	feature = unstable-backtraces-impl-std	backtrace
529999	\N	no_feature_used
474999	\N	no_feature_used
280947	\N	no_feature_used
442815	\N	no_feature_used
499215	\N	no_feature_used
322782	\N	no_feature_used
134717	\N	no_feature_used
598225	\N	no_feature_used
378409	\N	no_feature_used
577453	\N	no_feature_used
599782	feature = specialize	min_specialization
599782	feature = specialize	build_hasher_simple_hash_one
599782	feature = stdsimd	stdsimd
540974	\N	no_feature_used
312147	\N	no_feature_used
312146	\N	no_feature_used
279513	\N	no_feature_used
504459	\N	no_feature_used
247616	\N	no_feature_used
502274	\N	no_feature_used
485563	\N	no_feature_used
262258	\N	no_feature_used
565080	\N	no_feature_used
142754	not(feature = use_core)	no_core
577327	docsrs	doc_cfg
585917	\N	no_feature_used
552641	\N	no_feature_used
439401	\N	no_feature_used
512947	\N	no_feature_used
561151	\N	no_feature_used
324309	\N	no_feature_used
605361	\N	no_feature_used
278516	\N	no_feature_used
384845	\N	no_feature_used
599992	\N	no_feature_used
595417	\N	no_feature_used
580373	\N	no_feature_used
518019	\N	no_feature_used
552041	docsrs	doc_cfg
502504	\N	no_feature_used
606348	\N	no_feature_used
604597	\N	no_feature_used
451609	\N	no_feature_used
514792	\N	no_feature_used
569186	feature = docs	doc_cfg
286557	\N	no_feature_used
450305	\N	no_feature_used
515753	\N	no_feature_used
595390	docsrs	doc_cfg
484922	doc_cfg	doc_cfg
568415	\N	no_feature_used
515121	\N	no_feature_used
593101	docsrs	doc_cfg
536124	docsrs	doc_cfg
340599	docsrs	doc_cfg
495059	docsrs	doc_cfg
492511	\N	no_feature_used
587500	docsrs	doc_cfg
587500	docsrs	doc_notable_trait
370600	\N	no_feature_used
330870	\N	no_feature_used
590415	feature = nightly_derive	proc_macro_diagnostic
590416	feature = nightly	specialization
590416	feature = nightly	doc_cfg
516338	\N	no_feature_used
268271	not(use_fallback)	proc_macro_diagnostic
595360	\N	no_feature_used
162171	\N	no_feature_used
307438	\N	no_feature_used
564479	\N	no_feature_used
564989	\N	no_feature_used
560980	\N	no_feature_used
594381	\N	no_feature_used
602451	\N	no_feature_used
584890	docsrs	doc_cfg
588202	\N	no_feature_used
344623	\N	no_feature_used
411477	backtrace	backtrace
411477	feature = docs	doc_cfg
550843	\N	no_feature_used
550844	\N	no_feature_used
576844	\N	no_feature_used
510350	\N	no_feature_used
602927	\N	no_feature_used
515752	\N	no_feature_used
594102	docsrs	doc_cfg
594102	all(polyval_armv8,target_arch = aarch64)	stdsimd
557379	\N	no_feature_used
594271	docsrs	doc_cfg
259474	\N	no_feature_used
594056	docsrs	doc_cfg
593130	docsrs	doc_cfg
495695	\N	no_feature_used
531815	\N	no_feature_used
459714	\N	no_feature_used
541613	\N	no_feature_used
489445	\N	no_feature_used
604008	feature = nightly_portable_simd	portable_simd
587747	\N	no_feature_used
319758	\N	no_feature_used
409369	\N	no_feature_used
437070	\N	no_feature_used
477433	\N	no_feature_used
603846	backtrace	backtrace
603846	doc_cfg	doc_cfg
595413	\N	no_feature_used
595414	\N	no_feature_used
268269	\N	no_feature_used
606102	\N	no_feature_used
594050	\N	no_feature_used
603854	\N	no_feature_used
539074	\N	no_feature_used
532136	\N	no_feature_used
552260	docsrs	doc_cfg
537527	\N	no_feature_used
602453	\N	no_feature_used
487872	docsrs	doc_cfg
576823	\N	no_feature_used
595387	docsrs	doc_cfg
602250	os_str_bytes_docs_rs	doc_cfg
602250	all(target_vendor = fortanix,target_env = sgx)	sgx_platform
440044	\N	no_feature_used
520725	\N	no_feature_used
565037	\N	no_feature_used
500096	docsrs	doc_cfg
594903	docsrs	doc_cfg
536678	\N	no_feature_used
595697	\N	no_feature_used
540690	\N	no_feature_used
571156	docsrs	doc_cfg
538041	feature = nightly_slice_partition_dedup	slice_partition_dedup
538041	docs_rs	doc_cfg
579707	\N	no_feature_used
602919	\N	no_feature_used
389091	\N	no_feature_used
605360	\N	no_feature_used
527326	backtrace	backtrace
527326	doc_cfg	doc_cfg
442518	docsrs	doc_cfg
485894	\N	no_feature_used
422240	\N	no_feature_used
487880	docsrs	doc_cfg
343286	\N	no_feature_used
459712	\N	no_feature_used
556726	feature = const_fn	const_fn_trait_bound
318528	\N	no_feature_used
593502	\N	no_feature_used
473253	\N	no_feature_used
601135	\N	no_feature_used
297785	\N	no_feature_used
528459	\N	no_feature_used
324990	\N	no_feature_used
544148	\N	no_feature_used
269413	\N	no_feature_used
276806	\N	no_feature_used
591700	\N	no_feature_used
412928	\N	no_feature_used
591698	\N	no_feature_used
470277	\N	no_feature_used
589874	\N	no_feature_used
263841	\N	no_feature_used
385651	\N	no_feature_used
545688	docsrs	doc_cfg
569840	\N	no_feature_used
569841	\N	no_feature_used
375330	\N	no_feature_used
569842	\N	no_feature_used
569850	\N	no_feature_used
341696	\N	no_feature_used
518970	\N	no_feature_used
344857	docsrs	doc_cfg
582086	\N	no_feature_used
514878	docsrs	doc_cfg
271034	\N	no_feature_used
606369	docsrs	doc_cfg
578614	feature = fmt	const_mut_refs
578614	feature = constant_time_as_str	const_slice_from_raw_parts
578614	feature = __docsrs	doc_cfg
520468	value_bag_capture_const_type_id	const_type_id
536927	doc_cfg	doc_cfg
437720	\N	no_feature_used
570077	\N	no_feature_used
597946	\N	no_feature_used
550503	nightly	doc_cfg
597947	\N	no_feature_used
373254	\N	no_feature_used
545622	docsrs	doc_cfg
561153	docsrs	doc_cfg
545609	docsrs	doc_cfg
549721	docsrs	doc_cfg
549721	docsrs	doc_auto_cfg
549723	docsrs	doc_cfg
549723	docsrs	doc_auto_cfg
595101	\N	no_feature_used
334167	\N	no_feature_used
573779	docsrs	doc_cfg
585518	docsrs	doc_cfg
594382	\N	no_feature_used
476676	docsrs	doc_cfg
476676	docsrs	doc_auto_cfg
476676	docsrs	doc_cfg_hide
601354	doc_cfg	doc_cfg
601354	doc_cfg	doc_auto_cfg
543114	\N	no_feature_used
602795	\N	no_feature_used
543113	\N	no_feature_used
604886	\N	no_feature_used
433405	\N	no_feature_used
576146	docsrs	doc_cfg
549720	\N	no_feature_used
599214	docsrs	doc_auto_cfg
599214	docsrs	doc_cfg
595726	\N	no_feature_used
528845	\N	no_feature_used
602828	\N	no_feature_used
587329	\N	no_feature_used
587331	\N	no_feature_used
587328	\N	no_feature_used
587326	\N	no_feature_used
540081	\N	no_feature_used
587330	\N	no_feature_used
543895	\N	no_feature_used
458594	\N	no_feature_used
574616	\N	no_feature_used
542825	\N	no_feature_used
571799	\N	no_feature_used
580187	\N	no_feature_used
526435	\N	no_feature_used
586276	\N	no_feature_used
380325	feature = pattern	pattern
332104	\N	no_feature_used
133639	\N	no_feature_used
313500	target_os = wasi	wasi_ext
246479	\N	no_feature_used
119999	\N	no_feature_used
310558	\N	no_feature_used
61034	\N	no_feature_used
76517	\N	no_feature_used
260984	\N	no_feature_used
606236	all(test,exhaustive)	non_exhaustive_omitted_patterns_lint
391577	\N	no_feature_used
\.


--
-- PostgreSQL database dump complete
--


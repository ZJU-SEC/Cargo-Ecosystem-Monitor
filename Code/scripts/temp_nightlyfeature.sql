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
-- Name: tmp; Type: TABLE; Schema: public; Owner: postgres
--

CREATE TABLE public.tmp (
    id integer,
    feature character varying,
    nightly character varying
);


ALTER TABLE public.tmp OWNER TO postgres;

--
-- Data for Name: tmp; Type: TABLE DATA; Schema: public; Owner: postgres
--

COPY public.tmp (id, feature, nightly) FROM stdin;
601970	bench	test
27401	with-bench	test
383566	clippy	plugin
383566	bench	test
606237	unstable	never_type
579113	pattern	pattern
189280	nightly	test
516398	docsrs	doc_cfg
516398	docsrs	extended_key_value_attributes
603667	rustc-dep-of-std	link_cfg
603667	rustc-dep-of-std	no_core
153940	rustc-dep-of-std	no_core
499768	simd_support	stdsimd
606532	nightly	thread_id_value
575775	specialization	specialization
575775	may_dangle	dropck_eyepatch
479610	nightly	wasi_ext
426562	bench	test
542524	bench	test
542524	bench	unicode_internals
72203	clippy	plugin
72203	simd	platform_intrinsics
72203	simd	repr_simd
72203	simd_opt	cfg_target_feature
72203	simd_asm	asm
81896	nightly	drop_types_in_const
81896	nightly	const_fn
81896	nightly	const_unsafe_cell_new
129206	unstable	test
588937	unstable	trait_alias
524843	benchmark	test
524843	no-stdlib-ffi-binding,not(feature = std)	lang_items
166593	nightly	alloc
166593	nightly	read_initializer
166593	nightly	specialization
547509	doc-cfg	doc_cfg
271883	nightly	plugin
527627	simd-accel	stdsimd
527627	simd-accel	core_intrinsics
502244	simd	platform_intrinsics
502244	simd	repr_simd
229717	bench	test
537525	nightly-testing	plugin
501130	nightly	test
501130	nightly	doc_cfg
501130	simd_backend	stdsimd
586746	nightly	negative_impls
586746	nightly	auto_traits
181750	nightly	never_type
407181	no-stdlib-ffi-binding,not(feature = std)	lang_items
407181	no-stdlib-ffi-binding,not(feature = std)	panic_handler
540442	diagnostics	proc_macro_diagnostic
537528	unstable	proc_macro
537528	nightly-testing	plugin
547508	doc-cfg	doc_cfg
342488	unstable	allocator_api
342488	unstable	try_trait
342488	unstable	generator_trait
342488	unstable	never_type
342488	unstable	try_reserve
549724	nightly	auto_traits
549724	nightly	negative_impls
136895	exact-size-is-empty	exact_size_is_empty
136895	trusted-len	trusted_len
429330	bench	test
462561	unstable_const	const_ptr_offset_from
462561	unstable_const	const_refs_to_cell
579943	real_blackbox	test
136894	unstable	unicode_version
555081	simd	portable_simd
585744	nightly	test
585744	nightly	core_intrinsics
585744	nightly	dropck_eyepatch
585744	nightly	min_specialization
585744	nightly	extend_one
585744	nightly	allocator_api
585744	nightly	slice_ptr_get
585744	nightly	nonnull_slice_from_raw_parts
585744	nightly	maybe_uninit_array_assume_init
585744	nightly	build_hasher_simple_hash_one
542832	unstable-backtraces-impl-std	backtrace
599782	specialize	min_specialization
599782	specialize	build_hasher_simple_hash_one
599782	stdsimd	stdsimd
569186	docs	doc_cfg
590415	nightly_derive	proc_macro_diagnostic
590416	nightly	specialization
590416	nightly	doc_cfg
411477	docs	doc_cfg
604008	nightly_portable_simd	portable_simd
538041	nightly_slice_partition_dedup	slice_partition_dedup
556726	const_fn	const_fn_trait_bound
578614	fmt	const_mut_refs
578614	constant_time_as_str	const_slice_from_raw_parts
578614	__docsrs	doc_cfg
380325	pattern	pattern
\.


--
-- PostgreSQL database dump complete
--


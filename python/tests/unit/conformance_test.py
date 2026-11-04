# Copyright 2023-2026 Aerospike, Inc.
#
# Portions may be licensed to Aerospike, Inc. under one or more contributor
# license agreements WHICH ARE COMPATIBLE WITH THE APACHE LICENSE, VERSION 2.0.
#
# Licensed under the Apache License, Version 2.0 (the "License"); you may not
# use this file except in compliance with the License. You may obtain a copy of
# the License at http://www.apache.org/licenses/LICENSE-2.0
#
# Unless required by applicable law or agreed to in writing, software
# distributed under the License is distributed on an "AS IS" BASIS, WITHOUT
# WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied. See the
# License for the specific language governing permissions and limitations under
# the License.

"""Unit tests for the discovery conformance runner hooks."""

import pytest
from aerospike_native import (
    ClientPolicy,
    DnsSeedCandidateProvider,
    Host,
    PortMapAddressTranslator,
    ResultCode,
    SeedCandidateProvider,
    SeedMergePolicy,
    StaticSeedCandidateProvider,
    TranslationMissMode,
    _conformance_tls_name_matches,
    _conformance_translate,
    _ConformanceRefresher,
)
from aerospike_native.exceptions import ClientError

# Self-signed; CN cn-only.example.com, SANs node1.example.com,
# *.wild.example.com and 10.0.0.1.
CERTIFICATE = b"""-----BEGIN CERTIFICATE-----
MIIBqjCCAVCgAwIBAgIUYEjRCxXULi7gKTQPY3oxNZl2rcYwCgYIKoZIzj0EAwIw
HjEcMBoGA1UEAwwTY24tb25seS5leGFtcGxlLmNvbTAgFw0yNjExMDMyMTQ3Mjla
GA8yMTI2MTAxMDIxNDcyOVowHjEcMBoGA1UEAwwTY24tb25seS5leGFtcGxlLmNv
bTBZMBMGByqGSM49AgEGCCqGSM49AwEHA0IABCY5k3B1L2u9qilQabi9S8AmC+sn
9OZUjxE8wAYlH/Msc0EPKZSVXiEOsHr/mOx6I9UykPyCLJZO2nOmYX0y9A6jajBo
MB0GA1UdDgQWBBRZYcZNIVZpsZgSwcQMlXyc89ak/zAPBgNVHRMBAf8EBTADAQH/
MDYGA1UdEQQvMC2CEW5vZGUxLmV4YW1wbGUuY29tghIqLndpbGQuZXhhbXBsZS5j
b22HBAoAAAEwCgYIKoZIzj0EAwIDSAAwRQIhAOFioMLYoAXu/1GMa80kSikX7EkW
pjvkYZBi2B2oQACOAiBWnole0qFGcH0uKUh71+wnIETFLuCrXMZTZJgwlS2mUg==
-----END CERTIFICATE-----
"""

STATIC_SEED = Host("10.0.0.9", 3000)
FIRST = Host("10.0.0.1", 3000)
SECOND = Host("10.0.0.2", 3000)


class TestTranslate:
    """Address translation through the policy's translator and miss mode."""

    def test_ip_map_translates_and_a_miss_passes_through(self):
        policy = ClientPolicy()
        policy.ip_map = {"10.0.0.1": "203.0.113.1"}
        assert _conformance_translate(policy, "BB9", "10.0.0.1", 3000) == Host("203.0.113.1", 3000)
        missed = _conformance_translate(policy, "BB9", "10.0.0.2", 3000, tls_name="node2")
        assert missed == Host("10.0.0.2", 3000, "node2")

    def test_configured_translator_replaces_ip_map(self):
        policy = ClientPolicy()
        policy.ip_map = {"10.0.0.1": "203.0.113.1"}
        policy.address_translator = PortMapAddressTranslator({"10.0.0.1:3000": "lb.example.com:30001"})
        assert _conformance_translate(policy, "BB9", "10.0.0.1", 3000) == Host("lb.example.com", 30001)

    def test_strict_miss_raises_translation_miss(self):
        policy = ClientPolicy()
        policy.ip_map = {"10.0.0.1": "203.0.113.1"}
        policy.translation_on_miss = TranslationMissMode.STRICT
        with pytest.raises(ClientError, match="static-map has no mapping for node BB9 .* 10.0.0.2:3000") as e:
            _conformance_translate(policy, "BB9", "10.0.0.2", 3000)
        assert e.value.result_code == ResultCode.TRANSLATION_MISS


class TestRefresher:
    """Seed refreshes through the client's refresh and merge code."""

    @pytest.mark.parametrize(
        ("merge_policy", "snapshots"),
        [
            (SeedMergePolicy.MERGE, [[STATIC_SEED, FIRST], [STATIC_SEED, SECOND], [STATIC_SEED]]),
            (SeedMergePolicy.REPLACE, [[STATIC_SEED, FIRST], [SECOND], [SECOND]]),
            (SeedMergePolicy.DISCOVERY_ONLY, [[FIRST], [SECOND], [STATIC_SEED]]),
        ],
        ids=["merge", "replace", "discovery-only"],
    )
    def test_scripted_results_follow_the_merge_policy(self, merge_policy, snapshots):
        policy = ClientPolicy()
        policy.discovery_merge_policy = merge_policy
        refresher = _ConformanceRefresher.scripted(policy, [[FIRST], [SECOND], []], [STATIC_SEED])
        assert refresher.initial() == ([FIRST], None, snapshots[0])
        assert refresher.refresh() == ([SECOND], None, snapshots[1])
        assert refresher.refresh() == ([], None, snapshots[2])
        assert refresher.seeds == snapshots[2]

    def test_used_up_script_reports_an_error_and_keeps_the_seeds(self):
        refresher = _ConformanceRefresher.scripted(ClientPolicy(), [[FIRST]], [])
        refresher.initial()
        hosts, error, seeds = refresher.refresh()
        assert hosts is None
        assert isinstance(error, ClientError)
        assert seeds == [FIRST]

    def test_policy_provider_is_refreshed(self):
        policy = ClientPolicy()
        policy.seed_candidate_provider = StaticSeedCandidateProvider([FIRST])
        assert _ConformanceRefresher(policy, []).initial() == ([FIRST], None, [FIRST])

    def test_failing_provider_falls_back_to_the_static_seeds(self):
        policy = ClientPolicy()
        policy.seed_candidate_provider = DnsSeedCandidateProvider(["seeds.example.com"], servers=["127.0.0.1:1"])
        policy.discovery_refresh_timeout = 300
        hosts, error, seeds = _ConformanceRefresher(policy, [STATIC_SEED]).initial()
        assert hosts is None
        assert isinstance(error, ClientError)
        assert "seeds.example.com" in str(error)
        assert seeds == [STATIC_SEED]

    def test_policy_without_a_provider_is_rejected(self):
        with pytest.raises(ValueError, match="no seed_candidate_provider"):
            _ConformanceRefresher(ClientPolicy(), [])

    def test_subclass_provider_is_rejected(self):
        class RegistryProvider(SeedCandidateProvider):
            def refresh_seed_candidates(self, ctx):
                return [FIRST]

        policy = ClientPolicy()
        policy.seed_candidate_provider = RegistryProvider()
        with pytest.raises(ValueError, match="cannot be driven one refresh at a time"):
            _ConformanceRefresher(policy, [])


class TestTlsNameMatches:
    """Certificate name matching against the subject alternative names."""

    @pytest.mark.parametrize(
        ("tls_name", "matches"),
        [
            ("node1.example.com", True),
            ("NODE1.example.com", True),
            ("db.wild.example.com", True),
            ("a.db.wild.example.com", False),
            ("10.0.0.1", True),
            ("10.0.0.2", False),
            ("cn-only.example.com", False),
            ("not a name", False),
        ],
    )
    def test_matches_only_the_subject_alternative_names(self, tls_name, matches):
        assert _conformance_tls_name_matches(CERTIFICATE, tls_name) is matches

    def test_certificate_that_is_not_pem_is_rejected(self):
        with pytest.raises(ValueError, match="certificate is not PEM"):
            _conformance_tls_name_matches(b"not a certificate", "node1.example.com")

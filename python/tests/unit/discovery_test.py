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

"""Unit tests for seed discovery and address translation settings."""

from datetime import datetime, timezone

import pytest
from aerospike_native import (
    AwsAddressField,
    AwsCredentials,
    AwsEc2TagSeedCandidateProvider,
    ClientPolicy,
    DnsRecordType,
    DnsSeedCandidateProvider,
    Host,
    PassThroughAddressTranslator,
    PortMapAddressTranslator,
    ResultCode,
    SeedCandidateProvider,
    SeedMergePolicy,
    StaticMapAddressTranslator,
    StaticSeedCandidateProvider,
    TranslationMissMode,
    new_client_blocking,
)
from aerospike_native.exceptions import ClientError

# No DNS server listens here, so every query fails within the timeout.
DEAD_RESOLVER = "127.0.0.1:1"


def dns_provider(**kwargs) -> DnsSeedCandidateProvider:
    return DnsSeedCandidateProvider(["seeds.example.com"], servers=[DEAD_RESOLVER], **kwargs)


def aws_provider(**kwargs) -> AwsEc2TagSeedCandidateProvider:
    return AwsEc2TagSeedCandidateProvider("us-east-1", {"tag:aerospike-cluster": ["prod-a"]}, **kwargs)


class RegistryProvider(SeedCandidateProvider):
    """Fails every refresh, recording the context of each."""

    def __init__(self, registry_url):
        self.registry_url = registry_url
        self.contexts = []

    def refresh_seed_candidates(self, ctx):
        self.contexts.append(ctx)
        raise RuntimeError(f"{self.registry_url} is unavailable")


class TestClientPolicy:
    """Discovery settings on ClientPolicy."""

    def test_defaults_leave_discovery_off(self):
        policy = ClientPolicy()
        assert policy.seed_candidate_provider is None
        assert policy.address_translator is None
        assert policy.translation_on_miss == TranslationMissMode.PASS_THROUGH
        assert policy.discovery_merge_policy == SeedMergePolicy.MERGE
        assert policy.discovery_refresh_interval == 30000
        assert policy.discovery_tend_refresh_deadline == 100
        assert policy.discovery_refresh_timeout == 10000

    def test_zero_refresh_timeout_is_rejected(self):
        policy = ClientPolicy()
        with pytest.raises(ValueError, match="discovery_refresh_timeout must be greater than 0"):
            policy.discovery_refresh_timeout = 0
        assert policy.discovery_refresh_timeout == 10000

    def test_settings_round_trip(self):
        policy = ClientPolicy()
        policy.translation_on_miss = TranslationMissMode.STRICT
        policy.discovery_merge_policy = SeedMergePolicy.DISCOVERY_ONLY
        policy.discovery_refresh_interval = 60000
        policy.discovery_tend_refresh_deadline = 400
        assert policy.translation_on_miss == TranslationMissMode.STRICT
        assert policy.discovery_merge_policy == SeedMergePolicy.DISCOVERY_ONLY
        assert policy.discovery_refresh_interval == 60000
        assert policy.discovery_tend_refresh_deadline == 400

    @pytest.mark.parametrize(
        "provider",
        [StaticSeedCandidateProvider([Host("10.0.0.1", 3000)]), dns_provider(), aws_provider()],
        ids=["static", "dns", "aws"],
    )
    def test_seed_candidate_provider_round_trips(self, provider):
        policy = ClientPolicy()
        policy.seed_candidate_provider = provider
        assert type(policy.seed_candidate_provider) is type(provider)
        assert policy.seed_candidate_provider == provider
        policy.seed_candidate_provider = None
        assert policy.seed_candidate_provider is None

    def test_subclass_provider_is_kept_as_is(self):
        provider = RegistryProvider("https://registry.example.com")
        policy = ClientPolicy()
        policy.seed_candidate_provider = provider
        assert policy.seed_candidate_provider is provider

    def test_object_that_is_not_a_subclass_is_rejected(self):
        class LooksLikeAProvider:
            def refresh_seed_candidates(self, ctx):
                return []

        with pytest.raises(TypeError, match="or a SeedCandidateProvider subclass"):
            ClientPolicy().seed_candidate_provider = LooksLikeAProvider()

    @pytest.mark.parametrize(
        "translator",
        [
            StaticMapAddressTranslator({"10.0.0.1": "203.0.113.1"}),
            PortMapAddressTranslator({"10.0.0.1:3000": "lb.example.com:30001"}),
            PassThroughAddressTranslator(),
        ],
        ids=["static-map", "port-map", "pass-through"],
    )
    def test_address_translator_round_trips(self, translator):
        policy = ClientPolicy()
        policy.address_translator = translator
        assert type(policy.address_translator) is type(translator)
        assert policy.address_translator == translator
        policy.address_translator = None
        assert policy.address_translator is None

    def test_wrong_plugin_type_is_rejected(self):
        policy = ClientPolicy()
        with pytest.raises(TypeError, match="seed_candidate_provider must be"):
            policy.seed_candidate_provider = PassThroughAddressTranslator()
        with pytest.raises(TypeError, match="address_translator must be"):
            policy.address_translator = {"10.0.0.1": "203.0.113.1"}


class TestDnsSeedCandidateProvider:
    """DnsSeedCandidateProvider settings and their validation."""

    def test_defaults(self):
        provider = dns_provider()
        assert provider.names == ["seeds.example.com"]
        assert provider.record_type == DnsRecordType.A
        assert provider.port == 3000
        assert provider.tls_name is None
        assert provider.timeout == 80
        assert provider.servers == [DEAD_RESOLVER]

    def test_settings(self):
        provider = DnsSeedCandidateProvider(
            ["_aerospike._tcp.prod.example.com"],
            record_type=DnsRecordType.SRV,
            port=4000,
            tls_name="prod",
            timeout=50,
            servers=["10.20.0.2", "[2001:db8::1]:5353"],
        )
        assert provider.record_type == DnsRecordType.SRV
        assert (provider.port, provider.tls_name, provider.timeout) == (4000, "prod", 50)
        assert provider.servers == ["10.20.0.2", "[2001:db8::1]:5353"]

    @pytest.mark.parametrize(
        ("kwargs", "message"),
        [
            ({"names": []}, "at least one name is required"),
            ({"names": ["a.example.com", "A.example.com."]}, "duplicate name"),
            ({"names": ["a.example.com"], "port": 0}, "port: 0"),
            ({"names": ["a.example.com"], "timeout": 0}, "timeout: 0"),
            ({"names": ["a.example.com"], "servers": ["dns.example.com"]}, "expected an IP address"),
        ],
    )
    def test_invalid_setting_is_rejected(self, kwargs, message):
        with pytest.raises(ValueError, match=message):
            DnsSeedCandidateProvider(**kwargs)

    def test_a_bare_name_is_not_a_list_of_names(self):
        with pytest.raises(TypeError):
            DnsSeedCandidateProvider("seeds.example.com")


class TestAwsEc2TagSeedCandidateProvider:
    """AwsEc2TagSeedCandidateProvider settings, credentials and their validation."""

    def test_defaults(self):
        provider = aws_provider()
        assert provider.region == "us-east-1"
        assert provider.filters == {"tag:aerospike-cluster": ["prod-a"]}
        assert provider.address_field == AwsAddressField.PRIVATE_IP
        assert (provider.port, provider.tls_name, provider.endpoint) == (3000, None, None)
        assert (provider.timeout, provider.init_timeout) == (80, 10000)
        assert provider.credentials is None and provider.credentials_hook is None

    def test_settings(self):
        credentials = AwsCredentials("AKIAEXAMPLE", "secret")
        provider = AwsEc2TagSeedCandidateProvider(
            "eu-west-1",
            {"tag:aerospike-cluster": ["prod-a", "prod-b"], "subnet-id": ["subnet-0123"]},
            address_field=AwsAddressField.PUBLIC_DNS,
            port=4333,
            tls_name="prod",
            timeout=60,
            init_timeout=5000,
            endpoint="https://vpce-0123.ec2.eu-west-1.vpce.amazonaws.com",
            credentials=credentials,
        )
        assert list(provider.filters) == ["tag:aerospike-cluster", "subnet-id"]
        assert provider.address_field == AwsAddressField.PUBLIC_DNS
        assert (provider.port, provider.tls_name, provider.timeout, provider.init_timeout) == (4333, "prod", 60, 5000)
        assert provider.endpoint == "https://vpce-0123.ec2.eu-west-1.vpce.amazonaws.com"
        assert provider.credentials == credentials

    @pytest.mark.parametrize(
        ("region", "filters", "kwargs", "message"),
        [
            ("US", {"tag:a": ["b"]}, {}, "region: 'US'"),
            ("us-east-1", {}, {}, "at least one filter is required"),
            ("us-east-1", {"instance-state-name": ["running"]}, {}, "the provider adds instance-state-name=running"),
            ("us-east-1", {"tag:a": []}, {}, "filter 'tag:a' values: empty"),
            ("us-east-1", {"tag:a": ["b"]}, {"endpoint": "ftp://ec2.example.com"}, "endpoint: 'ftp://ec2.example.com'"),
            (
                "us-east-1",
                {"tag:a": ["b"]},
                {"credentials": AwsCredentials("AKIAEXAMPLE", "secret"), "credentials_hook": lambda: None},
                "credentials and credentials_hook cannot both be set",
            ),
        ],
    )
    def test_invalid_setting_is_rejected(self, region, filters, kwargs, message):
        with pytest.raises(ValueError, match=message):
            AwsEc2TagSeedCandidateProvider(region, filters, **kwargs)

    def test_hook_must_be_callable(self):
        with pytest.raises(TypeError, match="credentials_hook must be callable, not str"):
            aws_provider(credentials_hook="AKIAEXAMPLE")

    def test_credentials_hide_the_secret(self):
        expiration = datetime(2030, 1, 1, tzinfo=timezone.utc)
        credentials = AwsCredentials("AKIAEXAMPLE", "wJalrXUtnFEMI", "session-token", expiration)
        assert credentials.access_key_id == "AKIAEXAMPLE"
        assert credentials.expiration == expiration
        assert "wJalrXUtnFEMI" not in repr(credentials) and "session-token" not in repr(credentials)
        assert AwsCredentials("AKIAEXAMPLE", "secret").expiration is None
        with pytest.raises(ValueError, match="secret access key is empty"):
            AwsCredentials("AKIAEXAMPLE", "")

    @staticmethod
    def expired_sso_session():
        raise RuntimeError("SSO session expired")

    @staticmethod
    def bare_key_pair():
        return ("AKIAEXAMPLE", "secret")

    @pytest.mark.parametrize(
        ("hook", "message"),
        [
            (expired_sso_session, "RuntimeError: SSO session expired"),
            (bare_key_pair, "it returned tuple, not AwsCredentials"),
        ],
        ids=["raises", "wrong-type"],
    )
    def test_failing_hook_fails_construction(self, hook, message):
        policy = ClientPolicy()
        policy.seed_candidate_provider = aws_provider(credentials_hook=hook)
        with pytest.raises(ClientError) as exc_info:
            new_client_blocking(policy, [])
        assert exc_info.value.result_code == ResultCode.SEED_DISCOVERY_FAILED
        assert f"credentials hook failed: {message}" in str(exc_info.value)


class TestSeedCandidateProviderBase:
    """Defaults a SeedCandidateProvider subclass inherits."""

    def test_defaults(self):
        provider = SeedCandidateProvider()
        assert provider.needs_periodic_refresh() is False
        assert provider.close() is None

    def test_refresh_must_be_overridden(self):
        class Unfinished(SeedCandidateProvider):
            pass

        policy = ClientPolicy()
        policy.seed_candidate_provider = Unfinished()
        with pytest.raises(ClientError, match="NotImplementedError: SeedCandidateProvider subclasses must override"):
            new_client_blocking(policy, [])


class TestAddressTranslators:
    """Translator construction, validation and value semantics."""

    def test_maps_keep_their_entries_in_order(self):
        port_map = {"10.0.0.2:3000": "lb.example.com:30002", "BB9020011AC4201": "lb.example.com:30001"}
        assert list(PortMapAddressTranslator(port_map).port_map.items()) == list(port_map.items())
        ip_map = {"10.0.0.2": "203.0.113.2", "10.0.0.1": "203.0.113.1"}
        assert list(StaticMapAddressTranslator(ip_map).ip_map.items()) == list(ip_map.items())

    @pytest.mark.parametrize(
        ("key", "value", "reason"),
        [
            ("10.0.0.1:3000", "lb.example.com", "is not host:port"),
            ("10.0.0.1:3000", "lb.example.com:0", "out of range 1-65535"),
            ("2001:db8::1:3000", "lb.example.com:30001", "not in brackets"),
        ],
    )
    def test_invalid_port_map_entry_is_rejected(self, key, value, reason):
        with pytest.raises(ValueError, match=f"Invalid port-map entry {key}: {value}: .*{reason}"):
            PortMapAddressTranslator({key: value})

    def test_equal_when_their_mappings_are(self):
        assert PortMapAddressTranslator({"a:1": "b:2"}) == PortMapAddressTranslator({"a:1": "b:2"})
        assert PortMapAddressTranslator({"a:1": "b:2"}) != PortMapAddressTranslator({"a:1": "b:3"})
        assert PassThroughAddressTranslator() == PassThroughAddressTranslator()


class TestClientConstruction:
    """Discovery checks the client constructors make before any node is connected."""

    def test_failing_provider_with_no_static_seeds_fails_construction(self):
        policy = ClientPolicy()
        policy.seed_candidate_provider = dns_provider()
        with pytest.raises(ClientError) as exc_info:
            new_client_blocking(policy, [])
        assert exc_info.value.result_code == ResultCode.SEED_DISCOVERY_FAILED
        assert "provider dns failed" in str(exc_info.value)
        assert "seeds.example.com" in str(exc_info.value)

    def test_failing_subclass_provider_fails_construction(self):
        provider = RegistryProvider("https://registry.example.com")
        policy = ClientPolicy()
        policy.cluster_name = "prod"
        policy.seed_candidate_provider = provider
        with pytest.raises(ClientError) as exc_info:
            new_client_blocking(policy, [])
        assert exc_info.value.result_code == ResultCode.SEED_DISCOVERY_FAILED
        assert "provider RegistryProvider failed" in str(exc_info.value)
        assert "RuntimeError: https://registry.example.com is unavailable" in str(exc_info.value)
        [ctx] = provider.contexts
        assert (ctx.cluster_name, ctx.configured_seeds, ctx.tend_iteration) == ("prod", [], 0)

    def test_refresh_returning_the_wrong_type_fails_construction(self):
        class BareAddress(SeedCandidateProvider):
            def refresh_seed_candidates(self, ctx):
                return "10.0.0.1:3000"

        policy = ClientPolicy()
        policy.seed_candidate_provider = BareAddress()
        with pytest.raises(ClientError, match="refresh_seed_candidates returned str, not a list of Host"):
            new_client_blocking(policy, [])

    @pytest.mark.parametrize(
        ("settings", "message"),
        [
            ({"discovery_refresh_interval": 500}, "refresh interval 500 must be greater or equal to the tend interval 1000"),
            ({"discovery_tend_refresh_deadline": 501}, "deadline: 501. Must be > 0 and <= tend interval / 2"),
        ],
    )
    def test_invalid_refresh_settings_fail_construction(self, settings, message):
        policy = ClientPolicy()
        policy.seed_candidate_provider = dns_provider()
        for name, value in settings.items():
            setattr(policy, name, value)
        with pytest.raises(ClientError, match=message):
            new_client_blocking(policy, [Host("127.0.0.1", 1)])

    def test_provider_timeout_beyond_the_deadline_fails_construction(self):
        policy = ClientPolicy()
        policy.seed_candidate_provider = dns_provider(timeout=200)
        with pytest.raises(ClientError, match="timeout 200 must be less or equal to the discovery tend refresh deadline 100"):
            new_client_blocking(policy, [Host("127.0.0.1", 1)])

use httpmock::Method::GET;
use httpmock::{Mock, MockRef, MockServer};

mod common;

use goose::prelude::*;
use goose::GooseConfiguration;

const INDEX_PATH: &str = "/";
const ABOUT_PATH: &str = "/about.html";
const INDEX_KEY: usize = 0;
const ABOUT_KEY: usize = 1;

const METRICS_FILE: &str = "throttle-metrics.log";
const THROTTLE_REQUESTS: usize = 25;
const USERS: usize = 5;
const RUN_TIME: usize = 3;
const EXPECT_WORKERS: usize = 2;

pub async fn get_index(user: &GooseUser) -> GooseTaskResult {
    let _goose = user.get(INDEX_PATH).await?;
    Ok(())
}

pub async fn get_about(user: &GooseUser) -> GooseTaskResult {
    let _goose = user.get(ABOUT_PATH).await?;
    Ok(())
}

// All tests in this file run against common endpoints.
fn setup_mock_server_endpoints(server: &MockServer) -> Vec<MockRef> {
    let mut endpoints: Vec<MockRef> = Vec::new();

    // First set up INDEX_PATH, store in vector at INDEX_KEY.
    endpoints.push(
        Mock::new()
            .expect_method(GET)
            .expect_path(INDEX_PATH)
            .return_status(200)
            .create_on(&server),
    );
    // Next set up ABOUT_PATH, store in vector at ABOUT_KEY.
    endpoints.push(
        Mock::new()
            .expect_method(GET)
            .expect_path(ABOUT_PATH)
            .return_status(200)
            .create_on(&server),
    );

    endpoints
}

// Build configuration for a standalone load test.
fn common_build_configuration(
    server: &MockServer,
    metrics_file: &str,
    throttle_requests: usize,
    users: usize,
    run_time: usize,
    worker: Option<bool>,
    manager: Option<usize>,
) -> GooseConfiguration {
    if let Some(expect_workers) = manager {
        common::build_configuration(
            &server,
            vec![
                "--manager",
                "--expect-workers",
                &expect_workers.to_string(),
                "--users",
                &users.to_string(),
                "--hatch-rate",
                &users.to_string(),
                // Run the load test long enough to confirm the throttle is working correctly.
                "--run-time",
                &run_time.to_string(),
            ],
        )
    } else if worker.is_some() {
        common::build_configuration(
            &server,
            vec![
                "--worker",
                // Limit the maximum requests per second.
                "--throttle-requests",
                &throttle_requests.to_string(),
                // Write requests to file to confirm throttle is working.
                "--metrics-file",
                metrics_file,
            ],
        )
    } else {
        common::build_configuration(
            &server,
            vec![
                "--users",
                &users.to_string(),
                "--hatch-rate",
                &users.to_string(),
                // Run the load test long enough to confirm the throttle is working correctly.
                "--run-time",
                &run_time.to_string(),
                // Limit the maximum requests per second.
                "--throttle-requests",
                &throttle_requests.to_string(),
                // Write requests to file to confirm throttle is working.
                "--metrics-file",
                metrics_file,
            ],
        )
    }
}

// Common validation for the load tests in this file.
fn validate_throttle(
    mock_endpoints: &[MockRef],
    metrics_files: &[String],
    throttle_value: usize,
    previous_metrics_file_lines: Option<usize>,
) -> usize {
    // Verify that the metrics file was created and has the correct number of lines.
    let mut current_metrics_file_lines = 0;
    for metrics_file in metrics_files {
        assert!(std::path::Path::new(metrics_file).exists());
        current_metrics_file_lines += common::file_length(metrics_file);
    }

    // Confirm that we loaded the mock endpoints.
    assert!(mock_endpoints[INDEX_KEY].times_called() > 0);
    assert!(mock_endpoints[ABOUT_KEY].times_called() > 0);

    // Requests are made while GooseUsers are hatched, and then for RUN_TIME seconds.
    assert!(current_metrics_file_lines <= (RUN_TIME + 1) * throttle_value);

    if let Some(previous_lines) = previous_metrics_file_lines {
        // Verify the second load test generated more than 4x the load of the first test.
        assert!(current_metrics_file_lines > previous_lines * 4);
        // Verify the second load test generated less than 6x the load of the first test.
        assert!(current_metrics_file_lines < previous_lines * 6);
    }

    // Cleanup log file.
    for metrics_file in metrics_files {
        std::fs::remove_file(metrics_file).expect("failed to delete metrics log file");
    }

    // Return the number of lines in the current metrics file, allowing comparisons between
    // multiple tests with different throttle values.
    current_metrics_file_lines
}

// Returns the appropriate taskset needed to build this load test.
fn get_tasks() -> GooseTaskSet {
    taskset!("LoadTest")
        .register_task(task!(get_index))
        .register_task(task!(get_about))
}

#[test]
// Verify that the throttle limits the number of requests per second, and that increasing
// the throttle increases the number of requests per second.
fn test_throttle() {
    // Start the mock server.
    let server = MockServer::start();

    // Setup the endpoints needed for this test on the mock server.
    let mock_endpoints = setup_mock_server_endpoints(&server);

    // Build configuration.
    let configuration = common_build_configuration(
        &server,
        METRICS_FILE,
        THROTTLE_REQUESTS,
        USERS,
        RUN_TIME,
        None,
        None,
    );

    // Run the Goose Attack.
    common::run_load_test(
        common::build_load_test(configuration, &get_tasks(), None, None),
        None,
    );

    // Confirm that the load test was actually throttled.
    let test1_lines = validate_throttle(
        &mock_endpoints,
        &[METRICS_FILE.to_string()],
        THROTTLE_REQUESTS,
        None,
    );

    // Increase the throttle and run a second load test, so we can compare the difference
    // and confirm the throttle is actually working.
    let increased_throttle = THROTTLE_REQUESTS * 5;

    // Build a new configuration.
    let configuration = common_build_configuration(
        &server,
        METRICS_FILE,
        increased_throttle,
        USERS,
        RUN_TIME,
        None,
        None,
    );

    // Run the Goose Attack.
    common::run_load_test(
        common::build_load_test(configuration, &get_tasks(), None, None),
        None,
    );

    // Confirm that the load test was actually throttled, at an increased rate.
    let _ = validate_throttle(
        &mock_endpoints,
        &[METRICS_FILE.to_string()],
        increased_throttle,
        Some(test1_lines),
    );
}

#[test]
#[cfg_attr(not(feature = "gaggle"), ignore)]
// Verify that the throttle limits the number of requests per second even when running
// in Gaggle distributed load test, and that increasing the throttle increases the
// number of requests per second across the Gaggle.
fn test_throttle_gaggle() {
    // Multiple tests run together, so set a unique name.
    let metrics_file = "gaggle-".to_string() + METRICS_FILE;

    // Start the mock server.
    let server = MockServer::start();

    // Setup the endpoints needed for this test on the mock server.
    let mock_endpoints = setup_mock_server_endpoints(&server);

    // Each worker has the same identical configuration.
    let configuration =
        common_build_configuration(&server, "", THROTTLE_REQUESTS, 0, 0, Some(true), None);

    // Build the load test for the Workers.
    // Launch each worker in its own thread, storing the join handles.
    let mut worker_handles = Vec::new();
    let mut metrics_files = Vec::new();
    for i in 0..EXPECT_WORKERS {
        let mut worker_configuration = configuration.clone();
        worker_configuration.metrics_file = metrics_file.clone() + &i.to_string();
        metrics_files.push(worker_configuration.metrics_file.clone());
        let worker_goose_attack =
            common::build_load_test(worker_configuration.clone(), &get_tasks(), None, None);
        // Start worker instance of the load test.
        worker_handles.push(std::thread::spawn(move || {
            // Run the load test as configured.
            common::run_load_test(worker_goose_attack, None);
        }));
    }

    // Start manager instance in current thread and run a distributed load test.
    let manager_configuration = common_build_configuration(
        &server,
        &metrics_file,
        0,
        USERS,
        RUN_TIME,
        None,
        Some(EXPECT_WORKERS),
    );

    // Build the load test for the Manager.
    let manager_goose_attack =
        common::build_load_test(manager_configuration.clone(), &get_tasks(), None, None);

    // Run the Goose Attack.
    common::run_load_test(manager_goose_attack, Some(worker_handles));

    // Confirm that the load test was actually throttled.
    let test1_lines = validate_throttle(&mock_endpoints, &metrics_files, THROTTLE_REQUESTS, None);

    // Increase the throttle and run a second load test, so we can compare the difference
    // and confirm the throttle is actually working.
    let increased_throttle = THROTTLE_REQUESTS * 5;

    // Each worker has the same identical configuration.
    let configuration =
        common_build_configuration(&server, "", increased_throttle, 0, 0, Some(true), None);

    // Build the load test for the Workers.
    // Launch each worker in its own thread, storing the join handles.
    let mut worker_handles = Vec::new();
    let mut metrics_files = Vec::new();
    for i in 0..EXPECT_WORKERS {
        let mut worker_configuration = configuration.clone();
        worker_configuration.metrics_file = metrics_file.clone() + &i.to_string();
        metrics_files.push(worker_configuration.metrics_file.clone());
        let worker_goose_attack =
            common::build_load_test(worker_configuration.clone(), &get_tasks(), None, None);
        // Start worker instance of the load test.
        worker_handles.push(std::thread::spawn(move || {
            // Run the load test as configured.
            common::run_load_test(worker_goose_attack, None);
        }));
    }

    // Build the load test for the Manager.
    let manager_goose_attack =
        common::build_load_test(manager_configuration.clone(), &get_tasks(), None, None);

    // Run the Goose Attack.
    common::run_load_test(manager_goose_attack, Some(worker_handles));

    // Confirm that the load test was actually throttled, at an increased rate.
    let _ = validate_throttle(
        &mock_endpoints,
        &metrics_files,
        increased_throttle,
        Some(test1_lines),
    );
}
